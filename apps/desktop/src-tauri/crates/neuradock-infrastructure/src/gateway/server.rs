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

/// The protocol a relay's upstream endpoint actually speaks. Chosen by the
/// user when binding, because a relay URL alone does not reveal it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamKind {
    /// OpenAI Chat Completions (`/chat/completions`) — the common relay shape.
    OpenAiChat,
    /// OpenAI Responses (`/responses`) — native, matches Codex.
    OpenAiResponses,
    /// Anthropic Messages (`/v1/messages`) — native, matches Claude Code.
    Anthropic,
}

/// Upstream relay a client is bound to. Supplied by the app layer from an
/// account token or an independent key; the gateway never touches the
/// database.
#[derive(Debug, Clone)]
pub struct GatewayUpstream {
    /// Relay base URL, e.g. `https://relay.example` or `https://relay.example/v1`.
    pub base_url: String,
    /// Real relay API key, injected as a bearer token.
    pub api_key: String,
    /// Protocol the relay speaks, deciding passthrough vs translation.
    pub kind: UpstreamKind,
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
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    handle(state, GatewayClient::Codex, headers, body).await
}

async fn handle_anthropic(
    State(state): State<Arc<GatewayState>>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    handle(state, GatewayClient::ClaudeCode, headers, body).await
}

async fn handle(
    state: Arc<GatewayState>,
    client: GatewayClient,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let protocol = client.protocol();

    // Look up this client's upstream binding; each client is bound
    // independently and requests for unbound clients are rejected.
    let Some(upstream) = state.bindings.read().await.get(&client).cloned() else {
        log::warn!(
            "Gateway request for {} but no relay key is bound",
            client.display_name()
        );
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!(
                "no relay key is bound for {}; bind one in NeuraDock first",
                client.display_name()
            ),
        );
    };

    // Passthrough when the upstream already speaks the tool's protocol
    // (cc-switch's default path): forward verbatim with the real key, no
    // translation, streaming bytes proxied as-is.
    let passthrough = matches!(
        (protocol, upstream.kind),
        (InboundProtocol::Responses, UpstreamKind::OpenAiResponses)
            | (InboundProtocol::AnthropicMessages, UpstreamKind::Anthropic)
    );
    if passthrough {
        return forward_passthrough(&state, client, protocol, &upstream, &headers, body).await;
    }

    // Cross-protocol combinations other than "-> chat" are not implemented yet.
    if upstream.kind != UpstreamKind::OpenAiChat {
        return error_response(
            StatusCode::NOT_IMPLEMENTED,
            &format!(
                "{} with a {:?} upstream is not supported yet; bind a chat or native upstream",
                client.display_name(),
                upstream.kind
            ),
        );
    }

    let inbound: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            log::warn!(
                "Gateway received invalid JSON from {}: {}",
                client.display_name(),
                e
            );
            return error_response(StatusCode::BAD_REQUEST, &format!("invalid JSON: {}", e));
        }
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

    let url = build_upstream_url(&upstream.base_url, "/chat/completions");
    log::info!(
        "Gateway {} -> {} (convert, model={}, stream={})",
        client.display_name(),
        url,
        model,
        stream
    );
    let request = state
        .client
        .post(&url)
        .bearer_auth(&upstream.api_key)
        .json(&chat_body);

    let upstream = match request.send().await {
        Ok(resp) => resp,
        Err(e) => {
            log::error!("Gateway upstream request to {} failed: {}", url, e);
            return error_response(StatusCode::BAD_GATEWAY, &format!("upstream error: {}", e));
        }
    };

    if !upstream.status().is_success() {
        let status =
            StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
        let text = upstream.text().await.unwrap_or_default();
        log::error!(
            "Gateway upstream {} returned {}: {}",
            url,
            status.as_u16(),
            text.chars().take(500).collect::<String>()
        );
        return error_response(status, &text);
    }

    log::info!(
        "Gateway upstream {} responded OK, translating (stream={})",
        url,
        stream
    );
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
            Err(e) => {
                log::error!("Gateway got a non-JSON upstream body from {}: {}", url, e);
                error_response(
                    StatusCode::BAD_GATEWAY,
                    &format!("bad upstream body: {}", e),
                )
            }
        }
    }
}

/// Forward a request verbatim to a same-protocol upstream: real key injected,
/// optional model override, status and body (including SSE bytes) proxied
/// through unchanged. This is the default path for native upstreams, matching
/// cc-switch's non-converting forward.
async fn forward_passthrough(
    state: &GatewayState,
    client: GatewayClient,
    protocol: InboundProtocol,
    upstream: &GatewayUpstream,
    inbound_headers: &axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let endpoint = match protocol {
        InboundProtocol::Responses => "/responses",
        InboundProtocol::AnthropicMessages => "/v1/messages",
    };
    let url = build_upstream_url(&upstream.base_url, endpoint);
    log::info!(
        "Gateway {} -> {} (passthrough, model_override={:?})",
        client.display_name(),
        url,
        upstream.model
    );

    // Apply the model override without touching anything else in the body.
    let body_bytes = if let Some(model) = &upstream.model {
        match serde_json::from_slice::<Value>(&body) {
            Ok(mut v) => {
                v["model"] = Value::String(model.clone());
                serde_json::to_vec(&v).map(Into::into).unwrap_or(body)
            }
            Err(_) => body,
        }
    } else {
        body
    };

    let mut request = state
        .client
        .post(&url)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body_bytes);

    // Auth header shape depends on the upstream protocol.
    match upstream.kind {
        UpstreamKind::Anthropic => {
            request = request.header("x-api-key", &upstream.api_key);
            let version = inbound_headers
                .get("anthropic-version")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("2023-06-01");
            request = request.header("anthropic-version", version);
            if let Some(beta) = inbound_headers.get("anthropic-beta") {
                request = request.header("anthropic-beta", beta);
            }
        }
        UpstreamKind::OpenAiChat | UpstreamKind::OpenAiResponses => {
            request = request.bearer_auth(&upstream.api_key);
            if let Some(beta) = inbound_headers.get("openai-beta") {
                request = request.header("openai-beta", beta);
            }
        }
    }
    if let Some(accept) = inbound_headers.get(header::ACCEPT) {
        request = request.header(header::ACCEPT, accept);
    }

    let upstream_resp = match request.send().await {
        Ok(resp) => resp,
        Err(e) => {
            log::error!("Gateway passthrough to {} failed: {}", url, e);
            return error_response(StatusCode::BAD_GATEWAY, &format!("upstream error: {}", e));
        }
    };

    let status = StatusCode::from_u16(upstream_resp.status().as_u16())
        .unwrap_or(StatusCode::BAD_GATEWAY);
    if !status.is_success() {
        let text = upstream_resp.text().await.unwrap_or_default();
        log::error!(
            "Gateway passthrough upstream {} returned {}: {}",
            url,
            status.as_u16(),
            text.chars().take(500).collect::<String>()
        );
        return error_response(status, &text);
    }

    let content_type = upstream_resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();

    // Stream the upstream bytes through untouched (true incremental SSE).
    let byte_stream = upstream_resp.bytes_stream();
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from_stream(byte_stream))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

/// Combine a relay base URL with an API endpoint the way cc-switch's adapters
/// do: a base already ending in `/v1` joins directly, a bare origin gains
/// `/v1`, and a custom-prefix base (e.g. `https://x/openai`) joins without
/// inserting `/v1`. Duplicate `/v1/v1` segments collapse.
fn build_upstream_url(base_url: &str, endpoint: &str) -> String {
    let base = base_url.trim_end_matches('/');
    let ep = endpoint.trim_start_matches('/');

    let origin_only = url::Url::parse(base)
        .map(|u| u.path() == "/" || u.path().is_empty())
        .unwrap_or(false);

    let mut combined = if base.ends_with("/v1") {
        format!("{base}/{ep}")
    } else if origin_only {
        format!("{base}/v1/{ep}")
    } else {
        format!("{base}/{ep}")
    };

    while combined.contains("/v1/v1") {
        combined = combined.replace("/v1/v1", "/v1");
    }
    combined
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

    #[test]
    fn build_upstream_url_matches_cc_switch_rules() {
        // Bare origin gains /v1.
        assert_eq!(
            build_upstream_url("https://relay.example", "/chat/completions"),
            "https://relay.example/v1/chat/completions"
        );
        // Base already ending in /v1 joins directly (no doubled /v1).
        assert_eq!(
            build_upstream_url("https://relay.example/v1", "/chat/completions"),
            "https://relay.example/v1/chat/completions"
        );
        // Trailing slash is tolerated.
        assert_eq!(
            build_upstream_url("https://relay.example/v1/", "/responses"),
            "https://relay.example/v1/responses"
        );
        // Custom prefix does not get /v1 inserted.
        assert_eq!(
            build_upstream_url("https://relay.example/openai", "/chat/completions"),
            "https://relay.example/openai/chat/completions"
        );
    }

    /// Stand up a fake Chat Completions relay that echoes a fixed reply and
    /// records the request body it received.
    async fn spawn_fake_relay() -> (String, Arc<tokio::sync::Mutex<Option<Value>>>) {
        let seen = Arc::new(tokio::sync::Mutex::new(None));
        let seen_clone = seen.clone();
        let app = Router::new().route(
            "/v1/chat/completions",
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
                kind: UpstreamKind::OpenAiChat,
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
                kind: UpstreamKind::OpenAiChat,
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
            "/v1/chat/completions",
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

    #[tokio::test]
    async fn native_responses_upstream_is_passed_through_untouched() {
        // A relay that natively speaks Responses at /v1/responses. The request
        // must be forwarded verbatim (no chat conversion), only the key/model
        // adjusted.
        let seen = Arc::new(tokio::sync::Mutex::new(None));
        let seen_clone = seen.clone();
        let app = Router::new().route(
            "/v1/responses",
            post(move |Json(body): Json<Value>| {
                let seen = seen_clone.clone();
                async move {
                    *seen.lock().await = Some(body);
                    Json(json!({ "id": "resp_upstream", "object": "response", "status": "completed" }))
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
        let relay_url = format!("http://{}", addr);

        let mut map = HashMap::new();
        map.insert(
            GatewayClient::Codex,
            GatewayUpstream {
                base_url: relay_url,
                api_key: "sk-test".to_string(),
                kind: UpstreamKind::OpenAiResponses,
                model: Some("relay-model".to_string()),
            },
        );
        let gateway = start_gateway(Arc::new(RwLock::new(map)), 0).await.unwrap();

        let client = reqwest::Client::new();
        let resp: Value = client
            .post(format!("{}/responses", gateway.base_url()))
            .json(&json!({ "model": "gpt-5", "input": "ping", "instructions": "be terse" }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        // Upstream saw the original Responses body (instructions preserved,
        // not flattened into chat messages), with the model overridden.
        let upstream_body = seen.lock().await.clone().unwrap();
        assert_eq!(upstream_body["model"], "relay-model");
        assert_eq!(upstream_body["instructions"], "be terse");
        assert_eq!(upstream_body["input"], "ping");
        assert!(upstream_body.get("messages").is_none());

        // Response passed through untouched.
        assert_eq!(resp["id"], "resp_upstream");
        assert_eq!(resp["object"], "response");

        gateway.shutdown().await;
    }
}
