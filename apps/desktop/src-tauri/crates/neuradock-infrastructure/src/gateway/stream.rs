//! Streaming SSE translation for the gateway.
//!
//! The upstream relay returns Chat Completions SSE chunks
//! (`data: {"choices":[{"delta":{"content":"..."}}]}`). We translate each
//! chunk into the inbound protocol's event shape as it arrives, so the tool
//! sees incremental output rather than waiting for the whole response.
//!
//! These functions are pure: they take a parsed upstream chunk and return the
//! SSE event lines to forward. The transport layer owns byte framing.

use serde_json::{json, Value};

/// One server-sent event to forward downstream: an optional `event:` name and
/// a JSON `data:` payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
}

impl SseEvent {
    fn data_only(data: Value) -> Self {
        Self {
            event: None,
            data: data.to_string(),
        }
    }

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

/// Pull the text delta out of a Chat Completions streaming chunk, if present.
fn chat_chunk_text_delta(chunk: &Value) -> Option<&str> {
    chunk
        .get("choices")?
        .as_array()?
        .first()?
        .get("delta")?
        .get("content")?
        .as_str()
}

/// Whether a Chat Completions streaming chunk marks completion.
fn chat_chunk_finish(chunk: &Value) -> Option<&str> {
    chunk
        .get("choices")?
        .as_array()?
        .first()?
        .get("finish_reason")?
        .as_str()
}

// ---------------------------------------------------------------------------
// Chat SSE -> Responses SSE (Codex)
// ---------------------------------------------------------------------------

/// Translate one upstream Chat chunk into Responses SSE events.
pub fn chat_chunk_to_responses_events(chunk: &Value) -> Vec<SseEvent> {
    if let Some(text) = chat_chunk_text_delta(chunk) {
        return vec![SseEvent::data_only(json!({
            "type": "response.output_text.delta",
            "delta": text,
        }))];
    }
    Vec::new()
}

/// Terminal Responses events emitted once the upstream stream ends.
pub fn responses_stream_completion() -> Vec<SseEvent> {
    vec![SseEvent::data_only(json!({ "type": "response.completed" }))]
}

// ---------------------------------------------------------------------------
// Chat SSE -> Anthropic Messages SSE (Claude Code)
// ---------------------------------------------------------------------------

/// Anthropic's stream opens with message_start + a text content block before
/// any deltas. Emit those preamble events once at the start.
pub fn anthropic_stream_preamble(model: &str) -> Vec<SseEvent> {
    vec![
        SseEvent::named(
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
                    "usage": { "input_tokens": 0, "output_tokens": 0 },
                }
            }),
        ),
        SseEvent::named(
            "content_block_start",
            json!({
                "type": "content_block_start",
                "index": 0,
                "content_block": { "type": "text", "text": "" }
            }),
        ),
    ]
}

/// Translate one upstream Chat chunk into an Anthropic content delta.
pub fn chat_chunk_to_anthropic_events(chunk: &Value) -> Vec<SseEvent> {
    if let Some(text) = chat_chunk_text_delta(chunk) {
        return vec![SseEvent::named(
            "content_block_delta",
            json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": { "type": "text_delta", "text": text }
            }),
        )];
    }
    Vec::new()
}

/// Terminal Anthropic events emitted once the upstream stream ends. The finish
/// reason (if the upstream sent one) maps to Anthropic's stop_reason.
pub fn anthropic_stream_completion(finish_reason: Option<&str>) -> Vec<SseEvent> {
    let stop = match finish_reason {
        Some("tool_calls") => "tool_use",
        Some("length") => "max_tokens",
        _ => "end_turn",
    };
    vec![
        SseEvent::named(
            "content_block_stop",
            json!({ "type": "content_block_stop", "index": 0 }),
        ),
        SseEvent::named(
            "message_delta",
            json!({
                "type": "message_delta",
                "delta": { "stop_reason": stop },
                "usage": { "output_tokens": 0 }
            }),
        ),
        SseEvent::named("message_stop", json!({ "type": "message_stop" })),
    ]
}

/// Track the finish reason seen across streamed chunks so completion events can
/// report the right stop reason.
#[derive(Debug, Default)]
pub struct StreamFinish {
    reason: Option<String>,
}

impl StreamFinish {
    pub fn observe(&mut self, chunk: &Value) {
        if let Some(reason) = chat_chunk_finish(chunk) {
            self.reason = Some(reason.to_string());
        }
    }

    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_delta_becomes_responses_text_delta() {
        let chunk = json!({ "choices": [{ "delta": { "content": "hi" } }] });
        let events = chat_chunk_to_responses_events(&chunk);
        assert_eq!(events.len(), 1);
        assert!(events[0].data.contains("response.output_text.delta"));
        assert!(events[0].data.contains("hi"));
    }

    #[test]
    fn empty_delta_yields_no_events() {
        let chunk = json!({ "choices": [{ "delta": {} }] });
        assert!(chat_chunk_to_responses_events(&chunk).is_empty());
        assert!(chat_chunk_to_anthropic_events(&chunk).is_empty());
    }

    #[test]
    fn chat_delta_becomes_anthropic_content_delta() {
        let chunk = json!({ "choices": [{ "delta": { "content": "yo" } }] });
        let events = chat_chunk_to_anthropic_events(&chunk);
        assert_eq!(events[0].event.as_deref(), Some("content_block_delta"));
        assert!(events[0].data.contains("text_delta"));
        assert!(events[0].data.contains("yo"));
    }

    #[test]
    fn anthropic_preamble_opens_message_and_block() {
        let events = anthropic_stream_preamble("claude-3");
        assert_eq!(events[0].event.as_deref(), Some("message_start"));
        assert_eq!(events[1].event.as_deref(), Some("content_block_start"));
    }

    #[test]
    fn finish_tracker_maps_tool_calls_to_tool_use() {
        let mut finish = StreamFinish::default();
        finish.observe(&json!({ "choices": [{ "delta": {}, "finish_reason": "tool_calls" }] }));
        let events = anthropic_stream_completion(finish.reason());
        assert!(events[1].data.contains("tool_use"));
    }

    #[test]
    fn sse_wire_format_includes_event_name() {
        let ev = SseEvent::named("message_stop", json!({ "type": "message_stop" }));
        let wire = ev.to_wire();
        assert!(wire.starts_with("event: message_stop\ndata: "));
        assert!(wire.ends_with("\n\n"));
    }
}
