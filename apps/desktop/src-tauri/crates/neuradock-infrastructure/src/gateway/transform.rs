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

    if let Some(tools) = obj.get("tools").and_then(Value::as_array) {
        let mapped: Vec<Value> = tools.iter().map(responses_tool_to_chat).collect();
        if !mapped.is_empty() {
            chat.insert("tools".to_string(), Value::Array(mapped));
        }
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
        Some("reasoning") => {} // no portable Chat equivalent
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

fn responses_tool_to_chat(tool: &Value) -> Value {
    // Responses tools are already `{type:"function", name, description, parameters}`
    // at top level; Chat nests those under a `function` object.
    if tool.get("function").is_some() {
        return tool.clone();
    }
    json!({
        "type": "function",
        "function": {
            "name": tool.get("name").and_then(Value::as_str).unwrap_or(""),
            "description": tool.get("description").and_then(Value::as_str).unwrap_or(""),
            "parameters": tool.get("parameters").cloned().unwrap_or_else(|| json!({})),
        }
    })
}

/// Assemble a non-streaming OpenAI Responses response from a Chat Completions
/// response returned by the upstream relay.
pub fn chat_to_responses(chat: &Value, model: &str) -> Value {
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
        "id": "msg_neuradock_gateway",
        "role": "assistant",
        "content": [{ "type": "output_text", "text": text }],
    }));

    if let Some(calls) = message.and_then(|m| m.get("tool_calls")).and_then(Value::as_array) {
        for call in calls {
            let function = call.get("function");
            output.push(json!({
                "type": "function_call",
                "call_id": call.get("id").and_then(Value::as_str).unwrap_or(""),
                "name": function.and_then(|f| f.get("name")).and_then(Value::as_str).unwrap_or(""),
                "arguments": function.and_then(|f| f.get("arguments")).and_then(Value::as_str).unwrap_or("{}"),
            }));
        }
    }

    json!({
        "id": "resp_neuradock_gateway",
        "object": "response",
        "model": model,
        "status": "completed",
        "output": output,
        "usage": chat.get("usage").cloned().unwrap_or(Value::Null),
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

    if let Some(tools) = obj.get("tools").and_then(Value::as_array) {
        let mapped: Vec<Value> = tools.iter().map(anthropic_tool_to_chat).collect();
        if !mapped.is_empty() {
            chat.insert("tools".to_string(), Value::Array(mapped));
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
            if text.is_empty() { Value::Null } else { json!(text) },
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

fn anthropic_tool_to_chat(tool: &Value) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": tool.get("name").and_then(Value::as_str).unwrap_or(""),
            "description": tool.get("description").and_then(Value::as_str).unwrap_or(""),
            "parameters": tool.get("input_schema").cloned().unwrap_or_else(|| json!({})),
        }
    })
}

/// Assemble a non-streaming Anthropic Messages response from a Chat
/// Completions response returned by the upstream relay.
pub fn chat_to_anthropic(chat: &Value, model: &str) -> Value {
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
    if let Some(text) = message.and_then(|m| m.get("content")).and_then(Value::as_str) {
        if !text.is_empty() {
            content.push(json!({ "type": "text", "text": text }));
        }
    }
    if let Some(calls) = message.and_then(|m| m.get("tool_calls")).and_then(Value::as_array) {
        for call in calls {
            let function = call.get("function");
            let args = function
                .and_then(|f| f.get("arguments"))
                .and_then(Value::as_str)
                .and_then(|s| serde_json::from_str::<Value>(s).ok())
                .unwrap_or_else(|| json!({}));
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
        "id": "msg_neuradock_gateway",
        "type": "message",
        "role": "assistant",
        "model": model,
        "stop_reason": chat_finish_to_anthropic(finish),
        "content": content,
        "usage": {
            "input_tokens": usage.and_then(|u| u.get("prompt_tokens")).and_then(Value::as_i64).unwrap_or(0),
            "output_tokens": usage.and_then(|u| u.get("completion_tokens")).and_then(Value::as_i64).unwrap_or(0),
        },
    })
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
