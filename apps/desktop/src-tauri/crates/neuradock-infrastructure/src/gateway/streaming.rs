pub(crate) fn is_relay_billing_sse_block(block: &[u8]) -> bool {
    block
        .windows(b"event: billing_summary".len())
        .any(|window| window == b"event: billing_summary")
        || block
            .windows(b"\"type\":\"billing_summary\"".len())
            .any(|window| window == b"\"type\":\"billing_summary\"")
}

pub(crate) fn take_sse_block(buffer: &mut Vec<u8>) -> Option<Vec<u8>> {
    let lf = buffer.windows(2).position(|window| window == b"\n\n");
    let crlf = buffer.windows(4).position(|window| window == b"\r\n\r\n");
    let (offset, delimiter_len) = match (lf, crlf) {
        (Some(lf), Some(crlf)) if lf <= crlf => (lf, 2),
        (Some(_), Some(crlf)) => (crlf, 4),
        (Some(lf), None) => (lf, 2),
        (None, Some(crlf)) => (crlf, 4),
        (None, None) => return None,
    };
    Some(buffer.drain(..offset + delimiter_len).collect())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeSseObservation {
    pub valid: bool,
    pub terminal: bool,
}

pub(crate) fn inspect_native_sse_block(
    protocol: InboundProtocol,
    block: &[u8],
) -> Result<NativeSseObservation, String> {
    if is_relay_billing_sse_block(block) {
        return Ok(NativeSseObservation {
            valid: false,
            terminal: false,
        });
    }
    let text = std::str::from_utf8(block)
        .map_err(|_| "upstream SSE contained invalid UTF-8".to_string())?;
    let event_name = text.lines().find_map(|line| {
        line.trim_end_matches('\r')
            .strip_prefix("event:")
            .map(str::trim)
            .filter(|name| !name.is_empty())
    });
    let data = text
        .lines()
        .filter_map(|line| {
            line.trim_end_matches('\r')
                .strip_prefix("data:")
                .map(str::trim_start)
        })
        .collect::<Vec<_>>()
        .join("\n");
    if data.is_empty() {
        return Ok(NativeSseObservation {
            valid: false,
            terminal: false,
        });
    }
    if data == "[DONE]" {
        return Ok(NativeSseObservation {
            valid: true,
            terminal: true,
        });
    }
    let value: Value = serde_json::from_str(&data)
        .map_err(|_| "upstream SSE contained malformed JSON".to_string())?;
    let event_type = event_name.or_else(|| value.get("type").and_then(Value::as_str));
    if event_type == Some("error") || value.get("error").is_some() {
        return Err(value_error_message(&value)
            .unwrap_or_else(|| "upstream stream returned an error event".to_string()));
    }
    match protocol {
        InboundProtocol::AnthropicMessages => {
            let valid = matches!(
                event_type,
                Some(
                    "message_start"
                        | "content_block_start"
                        | "content_block_delta"
                        | "content_block_stop"
                        | "message_delta"
                        | "message_stop"
                )
            );
            Ok(NativeSseObservation {
                valid,
                terminal: event_type == Some("message_stop"),
            })
        }
        InboundProtocol::Responses => {
            let valid = event_type.is_some_and(|name| name.starts_with("response."));
            let terminal = matches!(
                event_type,
                Some("response.completed" | "response.failed" | "response.incomplete")
            );
            if event_type == Some("response.failed") {
                return Err(value
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("upstream Responses stream failed")
                    .to_string());
            }
            Ok(NativeSseObservation { valid, terminal })
        }
    }
}

pub(crate) fn native_stream_error(
    protocol: InboundProtocol,
    message: &str,
    request_id: &str,
) -> Vec<u8> {
    let message = sanitize_error_message(message);
    match protocol {
        InboundProtocol::AnthropicMessages => format!("event: error\ndata: {}\n\n", serde_json::json!({"type":"error","error":{"type":"api_error","message":message},"request_id":request_id})).into_bytes(),
        InboundProtocol::Responses => format!("event: response.failed\ndata: {}\n\n", serde_json::json!({"type":"response.failed","response":{"id":format!("resp_{}", request_id.replace('-', "")),"object":"response","status":"failed","output":[],"error":{"type":"upstream_error","message":message,"request_id":request_id}}})).into_bytes(),
    }
}
use super::reliability::{usage_from_json, GatewayTokenUsage};
use super::sanitization::{sanitize_error_message, value_error_message};
use super::transform::InboundProtocol;
use serde_json::Value;

pub(crate) fn usage_from_sse_block(block: &[u8]) -> GatewayTokenUsage {
    let text = String::from_utf8_lossy(block);
    let mut usage = GatewayTokenUsage::default();
    for line in text.lines() {
        let Some(payload) = line.trim().strip_prefix("data:") else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(payload.trim()) else {
            continue;
        };
        usage.merge(usage_from_json(&value));
        if let Some(message) = value.get("message") {
            usage.merge(usage_from_json(message));
        }
        if let Some(response) = value.get("response") {
            usage.merge(usage_from_json(response));
        }
    }
    usage
}
