//! Streaming SSE translation for the gateway.
//!
//! The upstream relay returns Chat Completions SSE chunks
//! (`data: {"choices":[{"delta":{"content":"..."}}]}`). We accumulate the whole
//! streamed response, then emit a protocol-correct SSE sequence for the inbound
//! tool. Buffering (rather than a naive per-chunk passthrough) is what makes the
//! Responses lifecycle valid: Codex rejects a stream that lacks the full
//! `response.created` -> item/part events -> `response.completed` sequence with
//! a populated `response` body. This mirrors the a8e responses gateway, a known
//! working Codex adapter.

use serde_json::{json, Value};

use super::reliability::{usage_from_json, GatewayTokenUsage};
use super::transform::{chat_to_anthropic, chat_to_responses, InboundProtocol};
use uuid::Uuid;

const RESPONSE_ID: &str = "resp_neuradock_gateway";
const MESSAGE_ITEM_ID: &str = "msg_neuradock_gateway";

/// One server-sent event to forward downstream: an optional `event:` name and
/// a JSON `data:` payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
}

impl SseEvent {
    fn named(event: &str, data: Value) -> Self {
        Self {
            event: Some(event.to_string()),
            data: data.to_string(),
        }
    }

    /// Serialize to SSE wire format (trailing blank line included).
    pub fn to_wire(&self) -> String {
        match &self.event {
            Some(name) => format!("event: {}\ndata: {}\n\n", name, self.data),
            None => format!("data: {}\n\n", self.data),
        }
    }
}

/// Upper bound on how many tool calls a single streamed response may
/// accumulate.
///
/// `tool_calls[].index` arrives verbatim from the upstream relay and is used to
/// size a `Vec`. Without a ceiling, one 40-byte chunk carrying
/// `{"index": 4000000000}` would drive an allocation of hundreds of gigabytes;
/// the allocator aborts on failure, which kills the entire desktop process
/// rather than just the offending request. The 4 MB pending-SSE cap does not
/// help here because the cost is driven by the value, not the payload size.
/// No real model emits anywhere near this many parallel tool calls.
const MAX_STREAMED_TOOL_CALLS: usize = 128;

/// A single reassembled tool call from a streamed response.
#[derive(Debug, Default, Clone)]
struct ToolCallAcc {
    id: String,
    name: String,
    arguments: String,
}

/// Accumulates a streamed Chat Completions response (text, tool calls, finish
/// reason) so the terminal events can carry a complete, correct body.
#[derive(Debug, Default)]
pub struct StreamAccumulator {
    text: String,
    tool_calls: Vec<ToolCallAcc>,
    finish_reason: Option<String>,
    usage: GatewayTokenUsage,
    tool_call_index_overflow: bool,
}

impl StreamAccumulator {
    /// True when the upstream sent a `tool_calls[].index` beyond
    /// [`MAX_STREAMED_TOOL_CALLS`]. The caller should fail the stream rather
    /// than hand back a silently incomplete tool-call set.
    pub fn tool_call_index_overflow(&self) -> bool {
        self.tool_call_index_overflow
    }

    /// Feed one parsed upstream Chat SSE chunk.
    pub fn observe(&mut self, chunk: &Value) {
        self.usage.merge(usage_from_json(chunk));
        let Some(choice) = chunk
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|c| c.first())
        else {
            return;
        };

        if let Some(text) = choice
            .get("delta")
            .and_then(|d| d.get("content"))
            .and_then(Value::as_str)
        {
            self.text.push_str(text);
        }

        if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
            self.finish_reason = Some(reason.to_string());
        }

        if let Some(calls) = choice
            .get("delta")
            .and_then(|d| d.get("tool_calls"))
            .and_then(Value::as_array)
        {
            for call in calls {
                let idx = call.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
                if idx >= MAX_STREAMED_TOOL_CALLS {
                    // Refuse to size an allocation from an unbounded upstream
                    // value. Flagged so the stream can be failed explicitly
                    // instead of returning a partial tool-call set.
                    self.tool_call_index_overflow = true;
                    continue;
                }
                while self.tool_calls.len() <= idx {
                    self.tool_calls.push(ToolCallAcc::default());
                }
                let acc = &mut self.tool_calls[idx];
                if let Some(id) = call.get("id").and_then(Value::as_str) {
                    if !id.is_empty() {
                        acc.id = id.to_string();
                    }
                }
                if let Some(function) = call.get("function") {
                    if let Some(name) = function.get("name").and_then(Value::as_str) {
                        if !name.is_empty() {
                            acc.name = name.to_string();
                        }
                    }
                    if let Some(args) = function.get("arguments").and_then(Value::as_str) {
                        acc.arguments.push_str(args);
                    }
                }
            }
        }
    }

    pub fn usage(&self) -> GatewayTokenUsage {
        self.usage
    }

    /// Reassemble the accumulated state into a non-streaming Chat Completions
    /// response, reusing the same body the non-streaming path produces.
    fn to_chat_response(&self) -> Value {
        let mut message = json!({ "role": "assistant", "content": self.text });
        if !self.tool_calls.is_empty() {
            let calls: Vec<Value> = self
                .tool_calls
                .iter()
                .map(|c| {
                    json!({
                        "id": c.id,
                        "type": "function",
                        "function": { "name": c.name, "arguments": c.arguments },
                    })
                })
                .collect();
            message["tool_calls"] = Value::Array(calls);
        }
        json!({
            "choices": [{
                "finish_reason": self.finish_reason.clone().unwrap_or_else(|| "stop".to_string()),
                "message": message,
            }],
            "usage": {
                "prompt_tokens": self.usage.input_tokens,
                "completion_tokens": self.usage.output_tokens,
                "prompt_tokens_details": {
                    "cached_tokens": self.usage.cache_read_tokens,
                }
            }
        })
    }
}

pub struct StreamBridge {
    protocol: InboundProtocol,
    model: String,
    response_id: String,
    message_id: String,
    accumulator: StreamAccumulator,
    lifecycle_started: bool,
    text_started: bool,
    saw_valid_chunk: bool,
}

impl StreamBridge {
    pub fn new(protocol: InboundProtocol, model: String) -> Self {
        let suffix = Uuid::new_v4().simple().to_string();
        Self {
            protocol,
            model,
            response_id: format!("resp_{suffix}"),
            message_id: format!("msg_{suffix}"),
            accumulator: StreamAccumulator::default(),
            lifecycle_started: false,
            text_started: false,
            saw_valid_chunk: false,
        }
    }

    pub fn observe(&mut self, chunk: &Value) -> Vec<SseEvent> {
        let Some(choice) = chunk
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
        else {
            self.accumulator.observe(chunk);
            return Vec::new();
        };

        self.saw_valid_chunk = true;
        let text_delta = choice
            .pointer("/delta/content")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(str::to_string);
        self.accumulator.observe(chunk);

        let mut events = self.start_events();
        if let Some(text) = text_delta {
            match self.protocol {
                InboundProtocol::Responses => {
                    if !self.text_started {
                        self.text_started = true;
                        events.extend(self.responses_text_start_events());
                    }
                    events.push(SseEvent::named(
                        "response.output_text.delta",
                        json!({
                            "type": "response.output_text.delta",
                            "item_id": self.message_id,
                            "output_index": 0,
                            "content_index": 0,
                            "delta": text,
                        }),
                    ));
                }
                InboundProtocol::AnthropicMessages => {
                    if !self.text_started {
                        self.text_started = true;
                        events.push(SseEvent::named(
                            "content_block_start",
                            json!({
                                "type": "content_block_start",
                                "index": 0,
                                "content_block": { "type": "text", "text": "" },
                            }),
                        ));
                    }
                    events.push(SseEvent::named(
                        "content_block_delta",
                        json!({
                            "type": "content_block_delta",
                            "index": 0,
                            "delta": { "type": "text_delta", "text": text },
                        }),
                    ));
                }
            }
        }
        events
    }

    pub fn finish(&mut self, terminal_seen: bool) -> Result<Vec<SseEvent>, String> {
        if !self.saw_valid_chunk {
            return Err("upstream stream ended without a valid Chat event".to_string());
        }
        if self.accumulator.tool_call_index_overflow() {
            return Err(format!(
                "upstream sent a tool_calls index beyond the supported maximum of {MAX_STREAMED_TOOL_CALLS}"
            ));
        }
        if !terminal_seen && self.accumulator.finish_reason.is_none() {
            return Err("upstream stream ended before a terminal event".to_string());
        }

        let mut events = self.start_events();
        match self.protocol {
            InboundProtocol::Responses => {
                if self.text_started {
                    let text = self.accumulator.text.clone();
                    events.push(SseEvent::named(
                        "response.output_text.done",
                        json!({
                            "type": "response.output_text.done",
                            "item_id": self.message_id,
                            "output_index": 0,
                            "content_index": 0,
                            "text": text,
                        }),
                    ));
                    events.push(SseEvent::named(
                        "response.content_part.done",
                        json!({
                            "type": "response.content_part.done",
                            "item_id": self.message_id,
                            "output_index": 0,
                            "content_index": 0,
                            "part": { "type": "output_text", "text": text },
                        }),
                    ));
                    events.push(SseEvent::named(
                        "response.output_item.done",
                        json!({
                            "type": "response.output_item.done",
                            "output_index": 0,
                            "item": {
                                "id": self.message_id,
                                "type": "message",
                                "status": "completed",
                                "role": "assistant",
                                "content": [{ "type": "output_text", "text": text }],
                            },
                        }),
                    ));
                }

                for (output_index, call) in
                    (usize::from(self.text_started)..).zip(self.accumulator.tool_calls.iter())
                {
                    append_function_call_events(&mut events, output_index, call);
                }

                let mut response =
                    chat_to_responses(&self.accumulator.to_chat_response(), &self.model);
                response["id"] = Value::String(self.response_id.clone());
                if let Some(message) = response
                    .get_mut("output")
                    .and_then(Value::as_array_mut)
                    .and_then(|output| output.first_mut())
                {
                    if message.get("type").and_then(Value::as_str) == Some("message") {
                        message["id"] = Value::String(self.message_id.clone());
                    }
                }
                events.push(SseEvent::named(
                    "response.completed",
                    json!({ "type": "response.completed", "response": response }),
                ));
            }
            InboundProtocol::AnthropicMessages => {
                if self.text_started {
                    events.push(SseEvent::named(
                        "content_block_stop",
                        json!({ "type": "content_block_stop", "index": 0 }),
                    ));
                }
                let message = chat_to_anthropic(&self.accumulator.to_chat_response(), &self.model);
                let mut index = usize::from(self.text_started);
                if let Some(content) = message.get("content").and_then(Value::as_array) {
                    for block in content {
                        if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                            append_anthropic_block(&mut events, index, block);
                            index += 1;
                        }
                    }
                }
                events.push(SseEvent::named(
                    "message_delta",
                    json!({
                        "type": "message_delta",
                        "delta": {
                            "stop_reason": message
                                .get("stop_reason")
                                .cloned()
                                .unwrap_or(json!("end_turn")),
                            "stop_sequence": Value::Null,
                        },
                        "usage": {
                            "output_tokens": self.accumulator.usage.output_tokens,
                        },
                    }),
                ));
                events.push(SseEvent::named(
                    "message_stop",
                    json!({ "type": "message_stop" }),
                ));
            }
        }
        Ok(events)
    }

    pub fn failure_events(&mut self, message: &str, request_id: &str) -> Vec<SseEvent> {
        let mut events = self.start_events();
        match self.protocol {
            InboundProtocol::Responses => events.push(SseEvent::named(
                "response.failed",
                json!({
                    "type": "response.failed",
                    "response": {
                        "id": self.response_id,
                        "object": "response",
                        "status": "failed",
                        "model": self.model,
                        "output": [],
                        "error": {
                            "message": message,
                            "type": "upstream_error",
                            "request_id": request_id,
                        },
                    },
                }),
            )),
            InboundProtocol::AnthropicMessages => events.push(SseEvent::named(
                "error",
                json!({
                    "type": "error",
                    "error": {
                        "type": "api_error",
                        "message": message,
                    },
                    "request_id": request_id,
                }),
            )),
        }
        events
    }

    pub fn usage(&self) -> GatewayTokenUsage {
        self.accumulator.usage()
    }

    fn start_events(&mut self) -> Vec<SseEvent> {
        if self.lifecycle_started {
            return Vec::new();
        }
        self.lifecycle_started = true;
        match self.protocol {
            InboundProtocol::Responses => vec![SseEvent::named(
                "response.created",
                json!({
                    "type": "response.created",
                    "response": {
                        "id": self.response_id,
                        "object": "response",
                        "status": "in_progress",
                        "model": self.model,
                        "output": [],
                    },
                }),
            )],
            InboundProtocol::AnthropicMessages => vec![SseEvent::named(
                "message_start",
                json!({
                    "type": "message_start",
                    "message": {
                        "id": self.message_id,
                        "type": "message",
                        "role": "assistant",
                        "model": self.model,
                        "content": [],
                        "stop_reason": Value::Null,
                        "stop_sequence": Value::Null,
                        "usage": {
                            "input_tokens": 0,
                            "output_tokens": 0,
                        },
                    },
                }),
            )],
        }
    }

    fn responses_text_start_events(&self) -> Vec<SseEvent> {
        vec![
            SseEvent::named(
                "response.output_item.added",
                json!({
                    "type": "response.output_item.added",
                    "output_index": 0,
                    "item": {
                        "id": self.message_id,
                        "type": "message",
                        "status": "in_progress",
                        "role": "assistant",
                        "content": [],
                    },
                }),
            ),
            SseEvent::named(
                "response.content_part.added",
                json!({
                    "type": "response.content_part.added",
                    "item_id": self.message_id,
                    "output_index": 0,
                    "content_index": 0,
                    "part": { "type": "output_text", "text": "" },
                }),
            ),
        ]
    }
}

// ---------------------------------------------------------------------------
// Accumulated response -> Responses SSE (Codex)
// ---------------------------------------------------------------------------

fn response_body(model: &str, status: &str, output: Vec<Value>) -> Value {
    json!({
        "id": RESPONSE_ID,
        "object": "response",
        "status": status,
        "model": model,
        "output": output,
    })
}

/// Build the full Responses SSE sequence Codex expects from an accumulated
/// stream: created -> (per output item added/delta/done) -> completed.
pub fn responses_sse_from_accumulator(model: &str, acc: &StreamAccumulator) -> Vec<SseEvent> {
    let mut events = vec![SseEvent::named(
        "response.created",
        json!({
            "type": "response.created",
            "response": response_body(model, "in_progress", Vec::new()),
        }),
    )];

    let mut output_index = 0usize;

    if !acc.text.is_empty() {
        append_message_events(&mut events, output_index, &acc.text);
        output_index += 1;
    }

    for call in &acc.tool_calls {
        append_function_call_events(&mut events, output_index, call);
        output_index += 1;
    }

    // The completed event carries the same body the non-streaming path returns.
    let final_response = chat_to_responses(&acc.to_chat_response(), model);
    events.push(SseEvent::named(
        "response.completed",
        json!({ "type": "response.completed", "response": final_response }),
    ));
    events
}

fn append_message_events(events: &mut Vec<SseEvent>, output_index: usize, text: &str) {
    events.push(SseEvent::named(
        "response.output_item.added",
        json!({
            "type": "response.output_item.added",
            "output_index": output_index,
            "item": {
                "id": MESSAGE_ITEM_ID,
                "type": "message",
                "status": "in_progress",
                "role": "assistant",
                "content": [],
            },
        }),
    ));
    events.push(SseEvent::named(
        "response.content_part.added",
        json!({
            "type": "response.content_part.added",
            "item_id": MESSAGE_ITEM_ID,
            "output_index": output_index,
            "content_index": 0,
            "part": { "type": "output_text", "text": "" },
        }),
    ));
    events.push(SseEvent::named(
        "response.output_text.delta",
        json!({
            "type": "response.output_text.delta",
            "item_id": MESSAGE_ITEM_ID,
            "output_index": output_index,
            "content_index": 0,
            "delta": text,
        }),
    ));
    events.push(SseEvent::named(
        "response.output_text.done",
        json!({
            "type": "response.output_text.done",
            "item_id": MESSAGE_ITEM_ID,
            "output_index": output_index,
            "content_index": 0,
            "text": text,
        }),
    ));
    events.push(SseEvent::named(
        "response.content_part.done",
        json!({
            "type": "response.content_part.done",
            "item_id": MESSAGE_ITEM_ID,
            "output_index": output_index,
            "content_index": 0,
            "part": { "type": "output_text", "text": text },
        }),
    ));
    events.push(SseEvent::named(
        "response.output_item.done",
        json!({
            "type": "response.output_item.done",
            "output_index": output_index,
            "item": {
                "id": MESSAGE_ITEM_ID,
                "type": "message",
                "status": "completed",
                "role": "assistant",
                "content": [{ "type": "output_text", "text": text }],
            },
        }),
    ));
}

fn append_function_call_events(
    events: &mut Vec<SseEvent>,
    output_index: usize,
    call: &ToolCallAcc,
) {
    let mut item =
        super::transform::chat_tool_call_to_responses_item(&call.id, &call.name, &call.arguments);
    if item.get("type").and_then(Value::as_str) == Some("function_call") {
        item["id"] = json!(call.id);
        item["status"] = json!("completed");
    }
    events.push(SseEvent::named(
        "response.output_item.added",
        json!({
            "type": "response.output_item.added",
            "output_index": output_index,
            "item": item,
        }),
    ));
    events.push(SseEvent::named(
        "response.output_item.done",
        json!({
            "type": "response.output_item.done",
            "output_index": output_index,
            "item": item,
        }),
    ));
}

// ---------------------------------------------------------------------------
// Accumulated response -> Anthropic Messages SSE (Claude Code)
// ---------------------------------------------------------------------------

/// Build the full Anthropic Messages SSE sequence from an accumulated stream:
/// message_start -> content blocks -> message_delta -> message_stop.
pub fn anthropic_sse_from_accumulator(model: &str, acc: &StreamAccumulator) -> Vec<SseEvent> {
    let message = chat_to_anthropic(&acc.to_chat_response(), model);
    let stop_reason = message
        .get("stop_reason")
        .and_then(Value::as_str)
        .unwrap_or("end_turn")
        .to_string();
    let content = message
        .get("content")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut events = vec![SseEvent::named(
        "message_start",
        json!({
            "type": "message_start",
            "message": {
                "id": "msg_neuradock_gateway",
                "type": "message",
                "role": "assistant",
                "model": model,
                "content": [],
                "stop_reason": Value::Null,
                "stop_sequence": Value::Null,
                "usage": message.get("usage").cloned().unwrap_or(json!({
                    "input_tokens": 0, "output_tokens": 0
                })),
            }
        }),
    )];

    for (index, block) in content.iter().enumerate() {
        append_anthropic_block(&mut events, index, block);
    }

    events.push(SseEvent::named(
        "message_delta",
        json!({
            "type": "message_delta",
            "delta": { "stop_reason": stop_reason, "stop_sequence": Value::Null },
            "usage": { "output_tokens": message.pointer("/usage/output_tokens").cloned().unwrap_or(json!(0)) },
        }),
    ));
    events.push(SseEvent::named(
        "message_stop",
        json!({ "type": "message_stop" }),
    ));
    events
}

fn append_anthropic_block(events: &mut Vec<SseEvent>, index: usize, block: &Value) {
    match block.get("type").and_then(Value::as_str) {
        Some("text") => {
            let text = block.get("text").and_then(Value::as_str).unwrap_or("");
            events.push(SseEvent::named(
                "content_block_start",
                json!({
                    "type": "content_block_start",
                    "index": index,
                    "content_block": { "type": "text", "text": "" },
                }),
            ));
            events.push(SseEvent::named(
                "content_block_delta",
                json!({
                    "type": "content_block_delta",
                    "index": index,
                    "delta": { "type": "text_delta", "text": text },
                }),
            ));
            events.push(SseEvent::named(
                "content_block_stop",
                json!({ "type": "content_block_stop", "index": index }),
            ));
        }
        Some("tool_use") => {
            events.push(SseEvent::named(
                "content_block_start",
                json!({
                    "type": "content_block_start",
                    "index": index,
                    "content_block": {
                        "type": "tool_use",
                        "id": block.get("id").cloned().unwrap_or(json!("")),
                        "name": block.get("name").cloned().unwrap_or(json!("")),
                        "input": {},
                    },
                }),
            ));
            events.push(SseEvent::named(
                "content_block_delta",
                json!({
                    "type": "content_block_delta",
                    "index": index,
                    "delta": {
                        "type": "input_json_delta",
                        "partial_json": block.get("input").map(|v| v.to_string()).unwrap_or_else(|| "{}".to_string()),
                    },
                }),
            ));
            events.push(SseEvent::named(
                "content_block_stop",
                json!({ "type": "content_block_stop", "index": index }),
            ));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acc_from(chunks: &[Value]) -> StreamAccumulator {
        let mut acc = StreamAccumulator::default();
        for chunk in chunks {
            acc.observe(chunk);
        }
        acc
    }

    /// A hostile or buggy relay must not be able to size our allocations.
    /// Before the bound, `index: 4_000_000_000` pushed ~288 GB of accumulators
    /// and aborted the whole desktop process.
    #[test]
    fn absurd_tool_call_index_is_rejected_not_allocated() {
        let acc = acc_from(&[json!({ "choices": [{ "delta": { "tool_calls": [
            { "index": 4_000_000_000u64, "id": "x", "function": { "name": "n", "arguments": "{}" } }
        ] } }] })]);

        assert!(acc.tool_call_index_overflow());
        assert!(
            acc.tool_calls.is_empty(),
            "no accumulator should have been allocated for an out-of-range index"
        );
    }

    #[test]
    fn tool_call_index_at_the_boundary_is_accepted() {
        let last_ok = MAX_STREAMED_TOOL_CALLS - 1;
        let acc = acc_from(&[json!({ "choices": [{ "delta": { "tool_calls": [
            { "index": last_ok, "id": "x", "function": { "name": "n", "arguments": "{}" } }
        ] } }] })]);

        assert!(!acc.tool_call_index_overflow());
        assert_eq!(acc.tool_calls.len(), MAX_STREAMED_TOOL_CALLS);

        let over = acc_from(&[json!({ "choices": [{ "delta": { "tool_calls": [
            { "index": MAX_STREAMED_TOOL_CALLS, "id": "x", "function": { "name": "n", "arguments": "{}" } }
        ] } }] })]);
        assert!(over.tool_call_index_overflow());
    }

    #[test]
    fn text_stream_produces_created_and_completed() {
        let acc = acc_from(&[
            json!({ "choices": [{ "delta": { "content": "he" } }] }),
            json!({ "choices": [{ "delta": { "content": "llo" } }] }),
            json!({ "choices": [{ "delta": {}, "finish_reason": "stop" }] }),
        ]);
        let events = responses_sse_from_accumulator("gpt-5", &acc);

        assert_eq!(
            events.first().unwrap().event.as_deref(),
            Some("response.created")
        );
        let last = events.last().unwrap();
        assert_eq!(last.event.as_deref(), Some("response.completed"));
        // Completed body carries the full text and a populated response object.
        assert!(last.data.contains("response"));
        assert!(last.data.contains("hello"));
        // The text delta carries the accumulated text.
        assert!(events
            .iter()
            .any(|e| e.event.as_deref() == Some("response.output_text.delta")
                && e.data.contains("hello")));
    }

    #[test]
    fn streamed_tool_call_fragments_reassemble() {
        let acc = acc_from(&[
            json!({ "choices": [{ "delta": { "tool_calls": [
                { "index": 0, "id": "call_1", "function": { "name": "get_time", "arguments": "{\"tz\"" } }
            ] } }] }),
            json!({ "choices": [{ "delta": { "tool_calls": [
                { "index": 0, "function": { "arguments": ":\"utc\"}" } }
            ] } }] }),
            json!({ "choices": [{ "delta": {}, "finish_reason": "tool_calls" }] }),
        ]);
        let events = responses_sse_from_accumulator("gpt-5", &acc);

        let completed = events.last().unwrap();
        assert!(completed.data.contains("function_call"));
        assert!(completed.data.contains("get_time"));
        // Arguments were concatenated across fragments.
        assert!(completed.data.contains("utc"));
    }

    #[test]
    fn anthropic_stream_has_message_start_and_stop() {
        let acc = acc_from(&[
            json!({ "choices": [{ "delta": { "content": "hi" } }] }),
            json!({ "choices": [{ "delta": {}, "finish_reason": "stop" }] }),
        ]);
        let events = anthropic_sse_from_accumulator("claude-3", &acc);

        assert_eq!(
            events.first().unwrap().event.as_deref(),
            Some("message_start")
        );
        assert_eq!(
            events.last().unwrap().event.as_deref(),
            Some("message_stop")
        );
        assert!(events
            .iter()
            .any(|e| e.event.as_deref() == Some("content_block_delta") && e.data.contains("hi")));
    }

    #[test]
    fn sse_wire_format_includes_event_name() {
        let ev = SseEvent::named(
            "response.completed",
            json!({ "type": "response.completed" }),
        );
        let wire = ev.to_wire();
        assert!(wire.starts_with("event: response.completed\ndata: "));
        assert!(wire.ends_with("\n\n"));
    }
}
