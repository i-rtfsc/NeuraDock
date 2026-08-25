use futures::StreamExt;
use serde_json::Value;
use std::time::Duration;

pub(super) async fn read_response_bytes(
    response: reqwest::Response,
    max_bytes: usize,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    let read = async move {
        let mut stream = response.bytes_stream();
        let mut body = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| "upstream body transport failed".to_string())?;
            if body.len().saturating_add(chunk.len()) > max_bytes {
                return Err(format!(
                    "upstream body exceeded the {} byte limit",
                    max_bytes
                ));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    };
    tokio::time::timeout(timeout, read).await.map_err(|_| {
        format!(
            "upstream body timed out after {} seconds",
            timeout.as_secs()
        )
    })?
}

pub(super) async fn read_response_text(
    response: reqwest::Response,
    max_bytes: usize,
    timeout: Duration,
) -> Result<String, String> {
    let bytes = read_response_bytes(response, max_bytes, timeout).await?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub(super) async fn read_response_json(
    response: reqwest::Response,
    max_bytes: usize,
    timeout: Duration,
) -> Result<Value, String> {
    let bytes = read_response_bytes(response, max_bytes, timeout).await?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("upstream returned malformed JSON: {error}"))
}
