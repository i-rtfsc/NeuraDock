use serde_json::Value;

use super::transform::InboundProtocol;

fn value_error_message(value: &Value) -> Option<String> {
    value
        .pointer("/error/message")
        .or_else(|| value.get("message"))
        .and_then(Value::as_str)
        .map(|message| message.to_string())
}

pub(super) fn validate_chat_response(value: &Value) -> Result<(), String> {
    if value.get("error").is_some() {
        return Err(value_error_message(value)
            .unwrap_or_else(|| "upstream returned a Chat error object".to_string()));
    }
    let choice = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .ok_or_else(|| "upstream Chat response contains no choices".to_string())?;
    if !choice.get("message").is_some_and(Value::is_object) {
        return Err("upstream Chat response contains no message".to_string());
    }
    Ok(())
}

pub(super) fn validate_conversion_request(
    protocol: InboundProtocol,
    value: &Value,
) -> Result<(), String> {
    let unsupported: &[&str] = match protocol {
        InboundProtocol::Responses => &["input_image", "input_file", "computer_screenshot"],
        InboundProtocol::AnthropicMessages => {
            &["image", "document", "thinking", "redacted_thinking"]
        }
    };
    if let Some(kind) = find_content_type(value, unsupported) {
        return Err(format!(
            "content type {kind} is not supported through Chat protocol conversion"
        ));
    }
    if protocol == InboundProtocol::Responses
        && find_object_by_type(value, "reasoning").is_some_and(|reasoning| {
            reasoning
                .get("encrypted_content")
                .and_then(Value::as_str)
                .is_some_and(|content| !content.is_empty())
                && reasoning.get("summary").is_none()
        })
    {
        return Err("encrypted reasoning without a summary cannot be preserved through Chat protocol conversion".to_string());
    }
    Ok(())
}

fn find_content_type<'a>(value: &'a Value, targets: &[&str]) -> Option<&'a str> {
    match value {
        Value::Object(object) => {
            if let Some(kind) = object.get("type").and_then(Value::as_str) {
                if targets.contains(&kind) {
                    return Some(kind);
                }
            }
            object
                .values()
                .find_map(|value| find_content_type(value, targets))
        }
        Value::Array(values) => values
            .iter()
            .find_map(|value| find_content_type(value, targets)),
        _ => None,
    }
}

fn find_object_by_type<'a>(value: &'a Value, target: &str) -> Option<&'a Value> {
    match value {
        Value::Object(object) => {
            if object.get("type").and_then(Value::as_str) == Some(target) {
                return Some(value);
            }
            object
                .values()
                .find_map(|value| find_object_by_type(value, target))
        }
        Value::Array(values) => values
            .iter()
            .find_map(|value| find_object_by_type(value, target)),
        _ => None,
    }
}

pub(super) fn validate_native_response(
    protocol: InboundProtocol,
    value: &Value,
) -> Result<(), String> {
    if value.get("error").is_some() {
        return Err(value_error_message(value)
            .unwrap_or_else(|| "upstream returned an error object".to_string()));
    }
    match protocol {
        InboundProtocol::AnthropicMessages
            if value.get("type").and_then(Value::as_str) != Some("message")
                || !value.get("content").is_some_and(Value::is_array) =>
        {
            Err("upstream returned an invalid Anthropic message".to_string())
        }
        InboundProtocol::Responses
            if value.get("status").and_then(Value::as_str) == Some("failed") =>
        {
            Err(value_error_message(value)
                .unwrap_or_else(|| "upstream Responses request failed".to_string()))
        }
        InboundProtocol::Responses
            if value.get("object").and_then(Value::as_str) != Some("response")
                || !value.get("output").is_some_and(Value::is_array) =>
        {
            Err("upstream returned an invalid Responses object".to_string())
        }
        _ => Ok(()),
    }
}
