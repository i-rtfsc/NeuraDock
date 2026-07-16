//! Embedded localhost HTTP gateway that adapts a Chat-Completions-only relay
//! to the protocol a tool expects (OpenAI Responses for Codex, Anthropic
//! Messages for Claude Code).
//!
//! The tool is pointed at `http://127.0.0.1:<port>` as its base URL. The
//! gateway receives the tool's request, translates it to Chat Completions,
//! forwards it to the real relay with the real key injected, then translates
//! the response back — including streamed SSE.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::RwLock;

use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use futures::StreamExt;
use serde_json::Value;
use tokio::net::TcpListener;

use super::stream::{
    anthropic_sse_from_accumulator, responses_sse_from_accumulator, StreamAccumulator,
};
use super::transform::{
    anthropic_to_chat, chat_to_anthropic, chat_to_responses, requested_model, responses_to_chat,
    wants_stream, InboundProtocol,
};

/// A tool the gateway can serve. Requests are routed to a client by URL path
/// (`/responses` -> Codex, `/v1/messages` -> Claude Code), and each client is
/// independently bound to its own upstream relay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GatewayClient {
    /// Codex CLI, speaking the OpenAI Responses protocol.
    Codex,
    /// Claude Code, speaking the Anthropic Messages protocol.
    ClaudeCode,
}

impl GatewayClient {
    /// The inbound protocol this client speaks.
    fn protocol(self) -> InboundProtocol {
        match self {
            GatewayClient::Codex => InboundProtocol::Responses,
            GatewayClient::ClaudeCode => InboundProtocol::AnthropicMessages,
        }
    }

    /// Human-readable name used in error payloads.
    pub fn display_name(self) -> &'static str {
        match self {
            GatewayClient::Codex => "Codex",
            GatewayClient::ClaudeCode => "Claude Code",
        }
    }
}

/// Upstream relay a client is bound to. Supplied by the app layer from an
/// account token or an independent key; the gateway never touches the
/// database.
#[derive(Debug, Clone)]
pub struct GatewayUpstream {
    /// Relay base URL, e.g. `https://relay.example`.
    pub base_url: String,
    /// Real relay API key, injected as a bearer token.
    pub api_key: String,
    /// Model id sent to the relay; when set, overrides the tool's model.
    pub model: Option<String>,
}

/// Live client -> upstream binding table, shared between the running server
/// and the app layer so bindings can change without restarting the server.
pub type GatewayBindings = Arc<RwLock<HashMap<GatewayClient, GatewayUpstream>>>;

struct GatewayState {
    bindings: GatewayBindings,
    client: reqwest::Client,
}

/// A running gateway bound to a localhost port.
pub struct GatewayHandle {
    addr: SocketAddr,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    join: tokio::task::JoinHandle<()>,
}

impl GatewayHandle {
    /// The `http://127.0.0.1:<port>` address tools should use as their base URL.
    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// The bound port.
    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    /// Stop the gateway and wait for the server task to finish.
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        let _ = self.join.await;
    }
}

/// Bind a gateway to `127.0.0.1:<port>` (port 0 picks a free port) and start
/// serving all clients present in `bindings`. Only loopback is ever bound.
pub async fn start_gateway(
    bindings: GatewayBindings,
    port: u16,
) -> Result<GatewayHandle, std::io::Error> {
    let state = Arc::new(GatewayState {
        bindings,
        client: reqwest::Client::new(),
    });

    let app = Router::new()
        .route("/responses", post(handle_responses))
        .route("/v1/responses", post(handle_responses))
        .route("/v1/messages", post(handle_anthropic))
        .with_state(state);

    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    let addr = listener.local_addr()?;

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let join = tokio::spawn(async move {
        let server = axum::serve(listener, app).with_graceful_shutdown(async {
            let _ = rx.await;
        });
        if let Err(e) = server.await {
            log::error!("Gateway server error: {}", e);
        }
    });

    log::info!("Provider gateway listening on http://{}", addr);
    Ok(GatewayHandle {
        addr,
        shutdown: Some(tx),
        join,
    })
}

async fn handle_responses(
    State(state): State<Arc<GatewayState>>,
    body: axum::body::Bytes,
) -> Response {
    handle(state, GatewayClient::Codex, body).await
}

async fn handle_anthropic(
    State(state): State<Arc<GatewayState>>,
    body: axum::body::Bytes,
) -> Response {
    handle(state, GatewayClient::ClaudeCode, body).await
}

async fn handle(
    state: Arc<GatewayState>,
    client: GatewayClient,
    body: axum::body::Bytes,
) -> Response {
    let protocol = client.protocol();

    // Look up this client's upstream binding; each client is bound
    // independently and requests for unbound clients are rejected.
    let Some(upstream) = state.bindings.read().await.get(&client).cloned() else {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!(
                "no relay key is bound for {}; bind one in NeuraDock first",
                client.display_name()
            ),
        );
    };

    let inbound: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("invalid JSON: {}", e)),
    };

    let stream = wants_stream(&inbound);
    let model = upstream
        .model
        .clone()
        .or_else(|| requested_model(&inbound))
        .unwrap_or_default();

    // Translate inbound -> Chat Completions.
    let model_override = upstream.model.as_deref();
    let mut chat_body = match protocol {
        InboundProtocol::Responses => responses_to_chat(&inbound, model_override),
        InboundProtocol::AnthropicMessages => anthropic_to_chat(&inbound, model_override),
    };
    // Force the stream flag to match what the tool asked for.
    chat_body["stream"] = Value::Bool(stream);
    if stream {
        chat_body["stream_options"] = serde_json::json!({ "include_usage": true });
    }

    let url = format!(
        "{}/chat/completions",
        upstream.base_url.trim_end_matches('/')
    );
    let request = state
        .client
        .post(&url)
        .bearer_auth(&upstream.api_key)
        .json(&chat_body);

    let upstream = match request.send().await {
        Ok(resp) => resp,
        Err(e) => {
            return error_response(StatusCode::BAD_GATEWAY, &format!("upstream error: {}", e))
        }
    };

    if !upstream.status().is_success() {
        let status =
            StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
        let text = upstream.text().await.unwrap_or_default();
        return error_response(status, &text);
    }

    if stream {
        stream_response(protocol, model, upstream)
    } else {
        match upstream.json::<Value>().await {
            Ok(chat) => {
                let translated = match protocol {
                    InboundProtocol::Responses => chat_to_responses(&chat, &model),
                    InboundProtocol::AnthropicMessages => chat_to_anthropic(&chat, &model),
                };
                json_response(StatusCode::OK, &translated)
            }
            Err(e) => error_response(
                StatusCode::BAD_GATEWAY,
                &format!("bad upstream body: {}", e),
            ),
        }
    }
}

/// Bridge the upstream Chat SSE stream into the inbound protocol's SSE stream.
///
/// The upstream stream is accumulated in full, then a protocol-correct event
/// sequence is emitted. Codex/Claude Code reject a partial or malformed
/// lifecycle (e.g. a Responses stream without `response.completed` carrying a
/// populated body), so a naive per-chunk passthrough is not enough.
fn stream_response(
    protocol: InboundProtocol,
    model: String,
    upstream: reqwest::Response,
) -> Response {
    let mut byte_stream = upstream.bytes_stream();

    let sse = async_stream::stream! {
        let mut buffer = String::new();
        let mut acc = StreamAccumulator::default();

        while let Some(chunk) = byte_stream.next().await {
            let Ok(bytes) = chunk else { break };
            buffer.push_str(&String::from_utf8_lossy(&bytes));

            // Process complete SSE lines (`data: {...}` separated by newlines).
            while let Some(newline) = buffer.find('\n') {
                let line = buffer[..newline].trim().to_string();
                buffer.drain(..=newline);

                let Some(payload) = line.strip_prefix("data:") else { continue };
                let payload = payload.trim();
                if payload.is_empty() {
                    continue;
                }
                if payload == "[DONE]" {
                    break;
                }
                let Ok(chat_chunk) = serde_json::from_str::<Value>(payload) else { continue };
                acc.observe(&chat_chunk);
            }
        }

        let events = match protocol {
            InboundProtocol::Responses => responses_sse_from_accumulator(&model, &acc),
            InboundProtocol::AnthropicMessages => anthropic_sse_from_accumulator(&model, &acc),
        };
        for event in events {
            yield Ok::<_, std::io::Error>(event.to_wire().into_bytes());
        }
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from_stream(sse))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn json_response(status: StatusCode, value: &Value) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/json")],
        value.to_string(),
    )
        .into_response()
}

fn error_response(status: StatusCode, message: &str) -> Response {
    let body = serde_json::json!({ "error": { "message": message } });
    json_response(status, &body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::post, Json, Router};
    use serde_json::json;

    /// Stand up a fake Chat Completions relay that echoes a fixed reply and
    /// records the request body it received.
    async fn spawn_fake_relay() -> (String, Arc<tokio::sync::Mutex<Option<Value>>>) {
        let seen = Arc::new(tokio::sync::Mutex::new(None));
        let seen_clone = seen.clone();
        let app = Router::new().route(
            "/chat/completions",
            post(move |Json(body): Json<Value>| {
                let seen = seen_clone.clone();
                async move {
                    *seen.lock().await = Some(body);
                    Json(json!({
                        "choices": [{
                            "finish_reason": "stop",
                            "message": { "content": "pong" }
                        }],
                        "usage": { "prompt_tokens": 1, "completion_tokens": 1 }
                    }))
                }
            }),
        );
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://{}", addr), seen)
    }

    fn bindings_for(client: GatewayClient, relay_url: String, model: &str) -> GatewayBindings {
        let mut map = HashMap::new();
        map.insert(
            client,
            GatewayUpstream {
                base_url: relay_url,
                api_key: "sk-test".to_string(),
                model: Some(model.to_string()),
            },
        );
        Arc::new(RwLock::new(map))
    }

    #[tokio::test]
    async fn responses_request_round_trips_through_gateway() {
        let (relay_url, seen) = spawn_fake_relay().await;
        let gateway = start_gateway(
            bindings_for(GatewayClient::Codex, relay_url, "relay-model"),
            0,
        )
        .await
        .unwrap();

        let client = reqwest::Client::new();
        let resp: Value = client
            .post(format!("{}/responses", gateway.base_url()))
            .json(&json!({ "model": "gpt-5", "input": "ping" }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        // Upstream saw a translated Chat request with the relay model injected.
        let upstream_body = seen.lock().await.clone().unwrap();
        assert_eq!(upstream_body["model"], "relay-model");
        assert_eq!(upstream_body["messages"][0]["content"], "ping");

        // Tool got a Responses-shaped reply.
        assert_eq!(resp["object"], "response");
        assert_eq!(resp["output"][0]["content"][0]["text"], "pong");

        gateway.shutdown().await;
    }

    #[tokio::test]
    async fn anthropic_request_round_trips_through_gateway() {
        let (relay_url, seen) = spawn_fake_relay().await;
        let gateway = start_gateway(
            bindings_for(GatewayClient::ClaudeCode, relay_url, "deepseek-chat"),
            0,
        )
        .await
        .unwrap();

        let client = reqwest::Client::new();
        let resp: Value = client
            .post(format!("{}/v1/messages", gateway.base_url()))
            .json(&json!({
                "model": "claude-3",
                "messages": [{ "role": "user", "content": "ping" }]
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        let upstream_body = seen.lock().await.clone().unwrap();
        assert_eq!(upstream_body["model"], "deepseek-chat");
        assert_eq!(upstream_body["messages"][0]["content"], "ping");

        assert_eq!(resp["type"], "message");
        assert_eq!(resp["content"][0]["text"], "pong");
        assert_eq!(resp["stop_reason"], "end_turn");

        gateway.shutdown().await;
    }

    #[tokio::test]
    async fn unbound_client_gets_service_unavailable() {
        let (relay_url, _seen) = spawn_fake_relay().await;
        // Only Codex is bound; a Claude Code request must be rejected.
        let gateway = start_gateway(bindings_for(GatewayClient::Codex, relay_url, "m"), 0)
            .await
            .unwrap();

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/v1/messages", gateway.base_url()))
            .json(&json!({ "messages": [] }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
        let body: Value = resp.json().await.unwrap();
        assert!(body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Claude Code"));

        gateway.shutdown().await;
    }

    #[tokio::test]
    async fn bindings_can_change_while_running() {
        let (relay_url, seen) = spawn_fake_relay().await;
        let bindings = bindings_for(GatewayClient::Codex, relay_url.clone(), "first");
        let gateway = start_gateway(bindings.clone(), 0).await.unwrap();

        // Rebind Codex to a different model without restarting.
        bindings.write().await.insert(
            GatewayClient::Codex,
            GatewayUpstream {
                base_url: relay_url,
                api_key: "sk-test".to_string(),
                model: Some("second".to_string()),
            },
        );

        let client = reqwest::Client::new();
        client
            .post(format!("{}/responses", gateway.base_url()))
            .json(&json!({ "input": "ping" }))
            .send()
            .await
            .unwrap();

        let upstream_body = seen.lock().await.clone().unwrap();
        assert_eq!(upstream_body["model"], "second");

        gateway.shutdown().await;
    }

    /// A fake relay that answers /chat/completions with an SSE stream, mimicking
    /// how a real chat-only relay streams deltas.
    async fn spawn_streaming_relay() -> String {
        let app = Router::new().route(
            "/chat/completions",
            post(|| async {
                let body = concat!(
                    "data: {\"choices\":[{\"delta\":{\"content\":\"po\"}}]}\n\n",
                    "data: {\"choices\":[{\"delta\":{\"content\":\"ng\"}}]}\n\n",
                    "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
                    "data: [DONE]\n\n",
                );
                ([(axum::http::header::CONTENT_TYPE, "text/event-stream")], body)
            }),
        );
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{}", addr)
    }

    #[tokio::test]
    async fn streamed_responses_end_with_completed_event() {
        let relay_url = spawn_streaming_relay().await;
        let gateway = start_gateway(
            bindings_for(GatewayClient::Codex, relay_url, "relay-model"),
            0,
        )
        .await
        .unwrap();

        let client = reqwest::Client::new();
        let sse = client
            .post(format!("{}/responses", gateway.base_url()))
            .json(&json!({ "input": "ping", "stream": true }))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        // The regression: Codex disconnects if the stream lacks a proper
        // lifecycle. It must open with response.created and close with a
        // response.completed carrying the assembled text.
        assert!(sse.contains("event: response.created"));
        assert!(sse.contains("event: response.completed"));
        assert!(sse.contains("response.output_text.delta"));
        assert!(sse.contains("pong"));

        gateway.shutdown().await;
    }

    #[tokio::test]
    async fn streamed_anthropic_ends_with_message_stop() {
        let relay_url = spawn_streaming_relay().await;
        let gateway = start_gateway(
            bindings_for(GatewayClient::ClaudeCode, relay_url, "deepseek-chat"),
            0,
        )
        .await
        .unwrap();

        let client = reqwest::Client::new();
        let sse = client
            .post(format!("{}/v1/messages", gateway.base_url()))
            .json(&json!({ "messages": [{ "role": "user", "content": "ping" }], "stream": true }))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        assert!(sse.contains("event: message_start"));
        assert!(sse.contains("event: message_stop"));
        assert!(sse.contains("pong"));

        gateway.shutdown().await;
    }
}
