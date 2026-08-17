//! Request/response translation for the embedded provider-adaptation gateway.
//!
//! The gateway lets a relay station that only speaks OpenAI Chat Completions
//! back a tool that speaks a different protocol. Two inbound protocols are
//! supported, both translated to and from Chat Completions:
//!
//! - OpenAI Responses (Codex CLI) `<->` Chat Completions
//! - Anthropic Messages (Claude Code) `<->` Chat Completions
//!
//! Everything here is pure and synchronous so it can be unit-tested without a
//! network or runtime. Streaming SSE translation lives in [`super::stream`].

use serde_json::{json, Map, Value};
use uuid::Uuid;

/// Chat-side proxy function name emulating the Responses `tool_search` tool,
/// which Chat Completions upstreams do not natively support. Mirrors the
/// approach used by cc-switch: the tool is presented to the upstream as a
/// plain function, and calls to it are translated back into
/// `tool_search_call` items for Codex.
pub(super) const TOOL_SEARCH_PROXY_NAME: &str = "tool_search";

/// Inbound protocol spoken by the tool that points at the gateway.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboundProtocol {
    /// OpenAI Responses API (`/responses`), used by Codex CLI.
    Responses,
    /// Anthropic Messages API (`/v1/messages`), used by Claude Code.
    AnthropicMessages,
}

/// Whether the caller asked for a streamed response.
pub fn wants_stream(body: &Value) -> bool {
    body.get("stream").and_then(Value::as_bool).unwrap_or(false)
}

/// Extract the requested model, if any.
pub fn requested_model(body: &Value) -> Option<String> {
    body.get("model")
        .and_then(Value::as_str)
        .map(str::to_string)
}

// ---------------------------------------------------------------------------
// Responses (Codex) -> Chat Completions
// ---------------------------------------------------------------------------

/// Convert an OpenAI Responses request body into a Chat Completions body.
pub fn responses_to_chat(body: &Value, model_override: Option<&str>) -> Value {
    let obj = body.as_object().cloned().unwrap_or_default();
    let mut chat = Map::new();

    let model = model_override
        .map(str::to_string)
        .or_else(|| obj.get("model").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_default();
    chat.insert("model".to_string(), Value::String(model));

    let mut messages = Vec::new();
    if let Some(instructions) = obj.get("instructions") {
        let text = instruction_text(instructions);
        if !text.is_empty() {
            messages.push(json!({ "role": "system", "content": text }));
        }
    }
    if let Some(input) = obj.get("input") {
        append_responses_input(input, &mut messages);
    }
    chat.insert("messages".to_string(), Value::Array(messages));

    let mut has_tools = false;
    if let Some(tools) = obj.get("tools").and_then(Value::as_array) {
        let mapped: Vec<Value> = tools.iter().filter_map(responses_tool_to_chat).collect();
        if !mapped.is_empty() {
            has_tools = true;
            chat.insert("tools".to_string(), Value::Array(mapped));
        }
    }
    // Only forward tool_choice when tools survived the filter; upstreams
    // reject a tool_choice with no tools.
    if has_tools {
        if let Some(mapped) = obj
            .get("tool_choice")
            .and_then(responses_tool_choice_to_chat)
        {
            chat.insert("tool_choice".to_string(), mapped);
        }
    }
    if let Some(parallel) = obj.get("parallel_tool_calls") {
        chat.insert("parallel_tool_calls".to_string(), parallel.clone());
    }

    // Passthrough of parameters that share a name and shape.
    for key in ["temperature", "top_p", "stream"] {
        if let Some(v) = obj.get(key) {
            chat.insert(key.to_string(), v.clone());
        }
    }
    if let Some(max) = obj
        .get("max_output_tokens")
        .or_else(|| obj.get("max_tokens"))
    {
        chat.insert("max_tokens".to_string(), max.clone());
    }

    Value::Object(chat)
}

fn responses_tool_choice_to_chat(choice: &Value) -> Option<Value> {
    if choice.is_string() {
        return Some(choice.clone());
    }
    match choice.get("type").and_then(Value::as_str) {
        Some("function") => Some(json!({
            "type": "function",
            "function": {
                "name": choice.get("name").and_then(Value::as_str).unwrap_or(""),
            },
        })),
        Some("tool_search") => Some(json!({
            "type": "function",
            "function": { "name": TOOL_SEARCH_PROXY_NAME },
        })),
        // A choice forcing a hosted tool that was dropped from the tools list
        // cannot be honored; drop it rather than fail the request.
        Some(_) => None,
        None => Some(choice.clone()),
    }
}

fn instruction_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|part| {
                part.get("text")
                    .and_then(Value::as_str)
                    .or_else(|| part.as_str())
            })
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n"),
        other => other.as_str().unwrap_or_default().to_string(),
    }
}

fn append_responses_input(input: &Value, messages: &mut Vec<Value>) {
    match input {
        Value::String(text) => messages.push(json!({ "role": "user", "content": text })),
        Value::Array(items) => {
            for item in items {
                append_responses_item(item, messages);
            }
        }
        Value::Object(_) => append_responses_item(input, messages),
        _ => {}
    }
}

fn append_responses_item(item: &Value, messages: &mut Vec<Value>) {
    match item.get("type").and_then(Value::as_str) {
        Some("function_call") => messages.push(json!({
            "role": "assistant",
            "content": null,
            "tool_calls": [{
                "id": item.get("call_id").and_then(Value::as_str).unwrap_or(""),
                "type": "function",
                "function": {
                    "name": item.get("name").and_then(Value::as_str).unwrap_or(""),
                    "arguments": item.get("arguments").and_then(Value::as_str).unwrap_or("{}"),
                }
            }]
        })),
        Some("function_call_output") => messages.push(json!({
            "role": "tool",
            "tool_call_id": item.get("call_id").and_then(Value::as_str).unwrap_or(""),
            "content": responses_output_to_string(item.get("output")),
        })),
        // tool_search history round-trips through the Chat proxy function:
        // the call becomes an assistant tool_call named `tool_search`, its
        // output a plain tool message.
        Some("tool_search_call") => messages.push(json!({
            "role": "assistant",
            "content": null,
            "tool_calls": [{
                "id": item.get("call_id").or_else(|| item.get("id")).and_then(Value::as_str).unwrap_or(""),
                "type": "function",
                "function": {
                    "name": TOOL_SEARCH_PROXY_NAME,
                    "arguments": tool_arguments_string(item.get("arguments")),
                }
            }]
        })),
        Some("tool_search_output") => messages.push(json!({
            "role": "tool",
            "tool_call_id": item.get("call_id").and_then(Value::as_str).unwrap_or(""),
            "content": responses_output_to_string(item.get("output")),
        })),
        Some("reasoning") => {
            let summary = item
                .get("summary")
                .and_then(Value::as_array)
                .map(|parts| {
                    parts
                        .iter()
                        .filter_map(|part| part.get("text").and_then(Value::as_str))
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();
            if !summary.is_empty() {
                messages.push(json!({ "role": "assistant", "content": summary }));
            }
        }
        _ => {
            let role = item
                .get("role")
                .and_then(Value::as_str)
                .map(responses_role_to_chat)
                .unwrap_or("user");
            let content = item
                .get("content")
                .map(responses_content_to_text)
                .unwrap_or_default();
            messages.push(json!({ "role": role, "content": content }));
        }
    }
}

fn responses_role_to_chat(role: &str) -> &'static str {
    match role {
        "system" | "developer" => "system",
        "assistant" => "assistant",
        "tool" => "tool",
        _ => "user",
    }
}

fn responses_content_to_text(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

fn responses_output_to_string(output: Option<&Value>) -> String {
    match output {
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// Chat tool_call arguments must be a JSON string; Responses tool_search_call
/// arguments arrive as an object.
fn tool_arguments_string(arguments: Option<&Value>) -> String {
    match arguments {
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
        None => "{}".to_string(),
    }
}

/// Parse Chat tool_call arguments back into the object form Responses
/// tool_search_call items carry.
pub(super) fn tool_arguments_object(arguments: &str) -> Value {
    serde_json::from_str::<Value>(arguments)
        .ok()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}))
}

/// Build the Responses output item for one Chat tool call: the proxy
/// `tool_search` function is restored to a `tool_search_call` item, everything
/// else stays a `function_call`.
pub(super) fn chat_tool_call_to_responses_item(
    call_id: &str,
    name: &str,
    arguments: &str,
) -> Value {
    if name == TOOL_SEARCH_PROXY_NAME {
        json!({
            "type": "tool_search_call",
            "call_id": call_id,
            "status": "completed",
            "execution": "client",
            "arguments": tool_arguments_object(arguments),
        })
    } else {
        json!({
            "type": "function_call",
            "call_id": call_id,
            "name": name,
            "arguments": arguments,
        })
    }
}

fn responses_tool_to_chat(tool: &Value) -> Option<Value> {
    match tool.get("type").and_then(Value::as_str) {
        Some("tool_search") => return Some(tool_search_proxy_tool()),
        Some("function") | Some("custom") | None => {}
        // Hosted tools (web_search, file_search, code_interpreter, ...) have
        // no Chat Completions equivalent and cannot be emulated; drop them so
        // the rest of the request still goes through.
        Some(other) => {
            log::info!("Dropping tool type {other} unsupported by Chat protocol conversion");
            return None;
        }
    }
    // Responses tools are already `{type:"function", name, description, parameters}`
    // at top level; Chat nests those under a `function` object.
    if tool.get("function").is_some() {
        return Some(tool.clone());
    }
    Some(json!({
        "type": "function",
        "function": {
            "name": tool.get("name").and_then(Value::as_str).unwrap_or(""),
            "description": tool.get("description").and_then(Value::as_str).unwrap_or(""),
            "parameters": tool.get("parameters").cloned().unwrap_or_else(|| json!({})),
        }
    }))
}

/// The Chat-side function tool standing in for the Responses `tool_search`
/// built-in; schema mirrors the arguments Codex sends on tool_search_call.
fn tool_search_proxy_tool() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": TOOL_SEARCH_PROXY_NAME,
            "description": "Search and load Codex tools, plugins, connectors, and MCP namespaces for the current task.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query for tools or connectors to load."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of tool groups to return."
                    }
                },
                "required": ["query"]
            }
        }
    })
}

/// Assemble a non-streaming OpenAI Responses response from a Chat Completions
/// response returned by the upstream relay.
pub fn chat_to_responses(chat: &Value, model: &str) -> Value {
    let suffix = Uuid::new_v4().simple().to_string();
    let response_id = format!("resp_{suffix}");
    let message_id = format!("msg_{suffix}");
    let choice = chat
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|c| c.first());
    let message = choice.and_then(|c| c.get("message"));
    let text = message
        .and_then(|m| m.get("content"))
        .and_then(Value::as_str)
        .unwrap_or("");

    let mut output = Vec::new();
    output.push(json!({
        "type": "message",
        "id": message_id,
        "role": "assistant",
        "content": [{ "type": "output_text", "text": text }],
    }));

    if let Some(calls) = message
        .and_then(|m| m.get("tool_calls"))
        .and_then(Value::as_array)
    {
        for call in calls {
            let function = call.get("function");
            output.push(chat_tool_call_to_responses_item(
                call.get("id").and_then(Value::as_str).unwrap_or(""),
                function
                    .and_then(|f| f.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                function
                    .and_then(|f| f.get("arguments"))
                    .and_then(Value::as_str)
                    .unwrap_or("{}"),
            ));
        }
    }

    json!({
        "id": response_id,
        "object": "response",
        "model": model,
        "status": "completed",
        "output": output,
        "usage": {
            "input_tokens": chat.pointer("/usage/prompt_tokens").cloned().unwrap_or(json!(0)),
            "output_tokens": chat.pointer("/usage/completion_tokens").cloned().unwrap_or(json!(0)),
            "total_tokens": chat.pointer("/usage/total_tokens").cloned().unwrap_or(json!(0)),
            "input_tokens_details": {
                "cached_tokens": chat.pointer("/usage/prompt_tokens_details/cached_tokens")
                    .cloned().unwrap_or(json!(0)),
            },
        },
    })
}

// ---------------------------------------------------------------------------
// Anthropic Messages (Claude Code) -> Chat Completions
// ---------------------------------------------------------------------------

/// Convert an Anthropic Messages request body into a Chat Completions body.
pub fn anthropic_to_chat(body: &Value, model_override: Option<&str>) -> Value {
    let obj = body.as_object().cloned().unwrap_or_default();
    let mut chat = Map::new();

    let model = model_override
        .map(str::to_string)
        .or_else(|| obj.get("model").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_default();
    chat.insert("model".to_string(), Value::String(model));

    let mut messages = Vec::new();
    if let Some(system) = obj.get("system") {
        let text = anthropic_system_text(system);
        if !text.is_empty() {
            messages.push(json!({ "role": "system", "content": text }));
        }
    }
    if let Some(items) = obj.get("messages").and_then(Value::as_array) {
        for item in items {
            append_anthropic_message(item, &mut messages);
        }
    }
    chat.insert("messages".to_string(), Value::Array(messages));

    let mut has_tools = false;
    if let Some(tools) = obj.get("tools").and_then(Value::as_array) {
        let mapped: Vec<Value> = tools.iter().filter_map(anthropic_tool_to_chat).collect();
        if !mapped.is_empty() {
            has_tools = true;
            chat.insert("tools".to_string(), Value::Array(mapped));
        }
    }
    // Only forward tool_choice when tools survived the filter; upstreams
    // reject a tool_choice with no tools.
    if has_tools {
        if let Some(choice) = obj.get("tool_choice") {
            let (tool_choice, parallel) = anthropic_tool_choice_to_chat(choice);
            chat.insert("tool_choice".to_string(), tool_choice);
            if let Some(parallel) = parallel {
                chat.insert("parallel_tool_calls".to_string(), Value::Bool(parallel));
            }
        }
    }

    for key in ["temperature", "top_p", "stream"] {
        if let Some(v) = obj.get(key) {
            chat.insert(key.to_string(), v.clone());
        }
    }
    if let Some(max) = obj.get("max_tokens") {
        chat.insert("max_tokens".to_string(), max.clone());
    }

    Value::Object(chat)
}

fn anthropic_tool_choice_to_chat(choice: &Value) -> (Value, Option<bool>) {
    let parallel = choice
        .get("disable_parallel_tool_use")
        .and_then(Value::as_bool)
        .map(|disabled| !disabled);
    let mapped = match choice.get("type").and_then(Value::as_str) {
        Some("auto") => json!("auto"),
        Some("any") => json!("required"),
        Some("none") => json!("none"),
        Some("tool") => json!({
            "type": "function",
            "function": {
                "name": choice.get("name").and_then(Value::as_str).unwrap_or(""),
            },
        }),
        _ => choice.clone(),
    };
    (mapped, parallel)
}

fn anthropic_system_text(system: &Value) -> String {
    match system {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n\n"),
        _ => String::new(),
    }
}

fn append_anthropic_message(item: &Value, messages: &mut Vec<Value>) {
    let role = item.get("role").and_then(Value::as_str).unwrap_or("user");
    let content = item.get("content");

    // Simple string content: pass straight through.
    if let Some(text) = content.and_then(Value::as_str) {
        messages.push(json!({ "role": role, "content": text }));
        return;
    }

    let Some(blocks) = content.and_then(Value::as_array) else {
        messages.push(json!({ "role": role, "content": "" }));
        return;
    };

    // Assistant messages may carry tool_use blocks -> Chat tool_calls.
    if role == "assistant" {
        let mut text = String::new();
        let mut tool_calls = Vec::new();
        for block in blocks {
            match block.get("type").and_then(Value::as_str) {
                Some("text") => text.push_str(block.get("text").and_then(Value::as_str).unwrap_or("")),
                Some("tool_use") => tool_calls.push(json!({
                    "id": block.get("id").and_then(Value::as_str).unwrap_or(""),
                    "type": "function",
                    "function": {
                        "name": block.get("name").and_then(Value::as_str).unwrap_or(""),
                        "arguments": block.get("input").map(|v| v.to_string()).unwrap_or_else(|| "{}".to_string()),
                    }
                })),
                _ => {}
            }
        }
        let mut msg = Map::new();
        msg.insert("role".to_string(), json!("assistant"));
        msg.insert(
            "content".to_string(),
            if text.is_empty() {
                Value::Null
            } else {
                json!(text)
            },
        );
        if !tool_calls.is_empty() {
            msg.insert("tool_calls".to_string(), Value::Array(tool_calls));
        }
        messages.push(Value::Object(msg));
        return;
    }

    // User messages may carry tool_result blocks -> Chat tool messages.
    let mut text = String::new();
    for block in blocks {
        match block.get("type").and_then(Value::as_str) {
            Some("text") => text.push_str(block.get("text").and_then(Value::as_str).unwrap_or("")),
            Some("tool_result") => {
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": block.get("tool_use_id").and_then(Value::as_str).unwrap_or(""),
                    "content": anthropic_tool_result_text(block.get("content")),
                }));
            }
            _ => {}
        }
    }
    if !text.is_empty() {
        messages.push(json!({ "role": "user", "content": text }));
    }
}

fn anthropic_tool_result_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(""),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

fn anthropic_tool_to_chat(tool: &Value) -> Option<Value> {
    // Anthropic server tools (web_search_20250305, computer_20250124, ...)
    // carry a versioned `type`; they run on Anthropic's side and have no Chat
    // Completions equivalent, so they are dropped rather than failing the
    // request. Client tools have no `type` or `type: "custom"`.
    match tool.get("type").and_then(Value::as_str) {
        None | Some("custom") => {}
        Some(other) => {
            log::info!("Dropping tool type {other} unsupported by Chat protocol conversion");
            return None;
        }
    }
    Some(json!({
        "type": "function",
        "function": {
            "name": tool.get("name").and_then(Value::as_str).unwrap_or(""),
            "description": tool.get("description").and_then(Value::as_str).unwrap_or(""),
            "parameters": tool.get("input_schema").cloned().unwrap_or_else(|| json!({})),
        }
    }))
}

/// Assemble a non-streaming Anthropic Messages response from a Chat
/// Completions response returned by the upstream relay.
pub fn chat_to_anthropic(chat: &Value, model: &str) -> Value {
    let message_id = format!("msg_{}", Uuid::new_v4().simple());
    let choice = chat
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|c| c.first());
    let message = choice.and_then(|c| c.get("message"));
    let finish = choice
        .and_then(|c| c.get("finish_reason"))
        .and_then(Value::as_str)
        .unwrap_or("stop");

    let mut content = Vec::new();
    let mut emitted_tool_use = false;
    if let Some(text) = message
        .and_then(|m| m.get("content"))
        .and_then(Value::as_str)
    {
        if !text.is_empty() {
            content.push(json!({ "type": "text", "text": text }));
        }
    }
    if let Some(calls) = message
        .and_then(|m| m.get("tool_calls"))
        .and_then(Value::as_array)
    {
        for call in calls {
            let function = call.get("function");
            let args = function
                .and_then(|f| f.get("arguments"))
                .and_then(Value::as_str)
                .and_then(|s| serde_json::from_str::<Value>(s).ok())
                .unwrap_or_else(|| json!({}));
            emitted_tool_use = true;
            content.push(json!({
                "type": "tool_use",
                "id": call.get("id").and_then(Value::as_str).unwrap_or(""),
                "name": function.and_then(|f| f.get("name")).and_then(Value::as_str).unwrap_or(""),
                "input": args,
            }));
        }
    }

    let usage = chat.get("usage");
    json!({
        "id": message_id,
        "type": "message",
        "role": "assistant",
        "model": model,
        "stop_reason": anthropic_stop_reason(finish, emitted_tool_use),
        "content": content,
        "usage": {
            "input_tokens": usage.and_then(|u| u.get("prompt_tokens")).and_then(Value::as_i64).unwrap_or(0),
            "output_tokens": usage.and_then(|u| u.get("completion_tokens")).and_then(Value::as_i64).unwrap_or(0),
            "cache_read_input_tokens": usage
                .and_then(|u| u.pointer("/prompt_tokens_details/cached_tokens"))
                .and_then(Value::as_i64)
                .unwrap_or(0),
        },
    })
}

/// Pick the Anthropic `stop_reason` for a converted Chat response.
///
/// `finish_reason` alone is not trustworthy here: many OpenAI-compatible
/// relays report `"stop"` on the very chunk that carries `tool_calls`. Claude
/// Code's agent loop only continues when `stop_reason == "tool_use"`, so
/// trusting `finish_reason` made it render tool calls without ever executing
/// them -- every tool call became a dead end. The presence of a `tool_use`
/// block in the content we just built is the authoritative signal.
pub(crate) fn anthropic_stop_reason(finish: &str, emitted_tool_use: bool) -> &'static str {
    if emitted_tool_use {
        return "tool_use";
    }
    chat_finish_to_anthropic(finish)
}

fn chat_finish_to_anthropic(finish: &str) -> &'static str {
    match finish {
        "tool_calls" => "tool_use",
        "length" => "max_tokens",
        _ => "end_turn",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A relay that reports `finish_reason: "stop"` alongside tool_calls must
    /// still yield `stop_reason: "tool_use"`, otherwise Claude Code renders the
    /// tool call but never executes it.
    #[test]
    fn tool_calls_force_tool_use_stop_reason_even_when_finish_reason_is_stop() {
        let chat = json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {
                    "content": "",
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": { "name": "read_file", "arguments": "{\"path\":\"a.txt\"}" }
                    }]
                }
            }],
            "usage": { "prompt_tokens": 1, "completion_tokens": 1 }
        });

        let out = chat_to_anthropic(&chat, "claude-3-5-sonnet");

        assert_eq!(out["stop_reason"], "tool_use");
        assert_eq!(out["content"][0]["type"], "tool_use");
        assert_eq!(out["content"][0]["name"], "read_file");
    }

    #[test]
    fn plain_text_replies_keep_their_mapped_stop_reason() {
        let chat = json!({
            "choices": [{
                "finish_reason": "stop",
                "message": { "content": "hello" }
            }]
        });
        assert_eq!(chat_to_anthropic(&chat, "m")["stop_reason"], "end_turn");

        let truncated = json!({
            "choices": [{
                "finish_reason": "length",
                "message": { "content": "hel" }
            }]
        });
        assert_eq!(
            chat_to_anthropic(&truncated, "m")["stop_reason"],
            "max_tokens"
        );
    }

    #[test]
    fn explicit_tool_calls_finish_reason_still_maps_to_tool_use() {
        assert_eq!(anthropic_stop_reason("tool_calls", false), "tool_use");
        assert_eq!(anthropic_stop_reason("stop", true), "tool_use");
        assert_eq!(anthropic_stop_reason("stop", false), "end_turn");
    }

    #[test]
    fn responses_string_input_becomes_user_message() {
        let body = json!({
            "model": "gpt-5",
            "instructions": "be terse",
            "input": "hello",
        });
        let chat = responses_to_chat(&body, Some("relay-model"));
        assert_eq!(chat["model"], "relay-model");
        let messages = chat["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "be terse");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"], "hello");
    }

    #[test]
    fn responses_function_call_and_output_roundtrip() {
        let body = json!({
            "input": [
                { "type": "function_call", "call_id": "c1", "name": "get_time", "arguments": "{}" },
                { "type": "function_call_output", "call_id": "c1", "output": "12:00" },
            ]
        });
        let chat = responses_to_chat(&body, None);
        let messages = chat["messages"].as_array().unwrap();
        assert_eq!(messages[0]["tool_calls"][0]["function"]["name"], "get_time");
        assert_eq!(messages[1]["role"], "tool");
        assert_eq!(messages[1]["tool_call_id"], "c1");
        assert_eq!(messages[1]["content"], "12:00");
    }

    #[test]
    fn hosted_tools_are_dropped_not_rejected() {
        let body = json!({
            "input": "hello",
            "tools": [
                { "type": "web_search" },
                { "type": "function", "name": "get_time", "parameters": {} },
            ],
        });
        let chat = responses_to_chat(&body, None);
        let tools = chat["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["function"]["name"], "get_time");
    }

    #[test]
    fn tool_choice_is_dropped_when_no_tools_survive() {
        let body = json!({
            "input": "hello",
            "tools": [{ "type": "web_search" }],
            "tool_choice": { "type": "web_search" },
        });
        let chat = responses_to_chat(&body, None);
        assert!(chat.get("tools").is_none());
        assert!(chat.get("tool_choice").is_none());
    }

    #[test]
    fn anthropic_server_tools_are_dropped() {
        let body = json!({
            "messages": [{ "role": "user", "content": "hi" }],
            "tools": [
                { "type": "web_search_20250305", "name": "web_search" },
                { "name": "get_time", "input_schema": { "type": "object" } },
            ],
        });
        let chat = anthropic_to_chat(&body, None);
        let tools = chat["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["function"]["name"], "get_time");
    }

    #[test]
    fn tool_search_tool_becomes_proxy_function() {
        let body = json!({
            "input": "hello",
            "tools": [{ "type": "tool_search" }],
            "tool_choice": { "type": "tool_search" },
        });
        let chat = responses_to_chat(&body, None);
        assert_eq!(chat["tools"][0]["type"], "function");
        assert_eq!(chat["tools"][0]["function"]["name"], "tool_search");
        assert_eq!(chat["tool_choice"]["function"]["name"], "tool_search");
    }

    #[test]
    fn tool_search_call_and_output_roundtrip() {
        let body = json!({
            "input": [
                {
                    "type": "tool_search_call",
                    "call_id": "ts1",
                    "arguments": { "query": "gmail", "limit": 5 },
                },
                { "type": "tool_search_output", "call_id": "ts1", "output": "found 2 tools" },
            ]
        });
        let chat = responses_to_chat(&body, None);
        let messages = chat["messages"].as_array().unwrap();
        let call = &messages[0]["tool_calls"][0];
        assert_eq!(call["function"]["name"], "tool_search");
        let arguments: Value =
            serde_json::from_str(call["function"]["arguments"].as_str().unwrap()).unwrap();
        assert_eq!(arguments["query"], "gmail");
        assert_eq!(messages[1]["role"], "tool");
        assert_eq!(messages[1]["tool_call_id"], "ts1");
        assert_eq!(messages[1]["content"], "found 2 tools");
    }

    #[test]
    fn chat_tool_search_call_restores_responses_item() {
        let chat = json!({
            "choices": [{ "message": {
                "content": null,
                "tool_calls": [{
                    "id": "call_ts",
                    "type": "function",
                    "function": { "name": "tool_search", "arguments": "{\"query\":\"gmail\"}" },
                }],
            }}],
        });
        let responses = chat_to_responses(&chat, "gpt-5");
        let item = &responses["output"][1];
        assert_eq!(item["type"], "tool_search_call");
        assert_eq!(item["call_id"], "call_ts");
        assert_eq!(item["execution"], "client");
        assert_eq!(item["arguments"]["query"], "gmail");
    }

    #[test]
    fn responses_tool_choice_and_parallel_flag_are_preserved() {
        let body = json!({
            "input": "hello",
            "tools": [{ "type": "function", "name": "search", "parameters": {} }],
            "tool_choice": { "type": "function", "name": "search" },
            "parallel_tool_calls": false,
        });
        let chat = responses_to_chat(&body, None);
        assert_eq!(chat["tool_choice"]["function"]["name"], "search");
        assert_eq!(chat["parallel_tool_calls"], false);
    }

    #[test]
    fn chat_response_becomes_responses_output() {
        let chat = json!({
            "choices": [{ "message": { "content": "hi there" } }],
            "usage": { "prompt_tokens": 3, "completion_tokens": 2 },
        });
        let responses = chat_to_responses(&chat, "gpt-5");
        assert_eq!(responses["output"][0]["type"], "message");
        assert_eq!(responses["output"][0]["content"][0]["text"], "hi there");
        assert_eq!(responses["status"], "completed");
    }

    #[test]
    fn anthropic_messages_with_system_and_tool_result() {
        let body = json!({
            "model": "claude-3",
            "system": "be nice",
            "messages": [
                { "role": "user", "content": "hi" },
                { "role": "assistant", "content": [
                    { "type": "tool_use", "id": "t1", "name": "search", "input": { "q": "x" } }
                ]},
                { "role": "user", "content": [
                    { "type": "tool_result", "tool_use_id": "t1", "content": "result" }
                ]},
            ]
        });
        let chat = anthropic_to_chat(&body, Some("deepseek-chat"));
        assert_eq!(chat["model"], "deepseek-chat");
        let messages = chat["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["content"], "hi");
        assert_eq!(messages[2]["tool_calls"][0]["function"]["name"], "search");
        assert_eq!(messages[3]["role"], "tool");
        assert_eq!(messages[3]["tool_call_id"], "t1");
        assert_eq!(messages[3]["content"], "result");
    }

    #[test]
    fn anthropic_tools_map_input_schema_to_parameters() {
        let body = json!({
            "messages": [],
            "tools": [{ "name": "f", "description": "d", "input_schema": { "type": "object" } }]
        });
        let chat = anthropic_to_chat(&body, None);
        assert_eq!(chat["tools"][0]["function"]["name"], "f");
        assert_eq!(chat["tools"][0]["function"]["parameters"]["type"], "object");
    }

    #[test]
    fn anthropic_tool_choice_maps_parallel_policy() {
        let body = json!({
            "messages": [{ "role": "user", "content": "hello" }],
            "tools": [{ "name": "search", "input_schema": { "type": "object" } }],
            "tool_choice": {
                "type": "tool",
                "name": "search",
                "disable_parallel_tool_use": true
            }
        });
        let chat = anthropic_to_chat(&body, None);
        assert_eq!(chat["tool_choice"]["function"]["name"], "search");
        assert_eq!(chat["parallel_tool_calls"], false);
    }

    #[test]
    fn chat_response_becomes_anthropic_message() {
        let chat = json!({
            "choices": [{
                "finish_reason": "tool_calls",
                "message": {
                    "content": "sure",
                    "tool_calls": [{ "id": "t1", "function": { "name": "f", "arguments": "{\"a\":1}" } }]
                }
            }],
            "usage": { "prompt_tokens": 5, "completion_tokens": 4 },
        });
        let msg = chat_to_anthropic(&chat, "claude-3");
        assert_eq!(msg["stop_reason"], "tool_use");
        assert_eq!(msg["content"][0]["text"], "sure");
        assert_eq!(msg["content"][1]["type"], "tool_use");
        assert_eq!(msg["content"][1]["input"]["a"], 1);
        assert_eq!(msg["usage"]["input_tokens"], 5);
    }
}
