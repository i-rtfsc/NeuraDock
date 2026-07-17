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
    /// Overwrite the incoming request's client fingerprint headers so the relay
    /// sees a canonical Codex / Claude Code request regardless of the real
    /// caller. Lets any agent (or a bare script) reach a relay that only admits
    /// recognised clients.
    pub impersonate: bool,
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
    // using native protocol passthrough: forward with the real key, no
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
    // Forward the tool's headers (user-agent, originator, session id, ...) so
    // the relay's client fingerprint check passes, replacing only auth. The
    // body is our translated chat body, so drop the client's content-length.
    let chat_bytes = serde_json::to_vec(&chat_body).unwrap_or_default();
    let request = forward_headers(state.client.post(&url), &headers, &upstream, protocol)
        .header(header::CONTENT_TYPE, "application/json")
        .body(chat_bytes);

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
/// a standard non-converting forward.
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

    // Parse only enough request metadata to choose the correct downstream
    // response mode. Non-streaming responses are buffered and only
    // a live byte stream for stream:true; treating every 200 body as a stream
    // turns ordinary JSON into a chunked response that Claude Code can reject.
    let inbound_json = serde_json::from_slice::<Value>(&body).ok();
    let stream = inbound_json
        .as_ref()
        .and_then(|value| value.get("stream"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    // Apply the model override without touching anything else in the body.
    let body_bytes = if let Some(model) = &upstream.model {
        // Claude Code's `[1m]` suffix is a local capability marker. Claude
        // Code turns it into an anthropic-beta value before sending the
        // request; native Anthropic relays expect the real model id without
        // the suffix. Keep this defensive strip for bindings restored from an
        // older NeuraDock version that persisted the marker upstream.
        let outbound_model = if protocol == InboundProtocol::AnthropicMessages {
            strip_one_m_model_marker(model)
        } else {
            model.as_str()
        };
        match inbound_json {
            Some(mut v) => {
                v["model"] = Value::String(outbound_model.to_string());
                serde_json::to_vec(&v).map(Into::into).unwrap_or(body)
            }
            None => body,
        }
    } else {
        body
    };

    // Forward the tool's original headers because relays
    // fingerprint the client (user-agent, originator, session id, etc.) and
    // reject requests that don't look like a real Codex / Claude Code. We keep
    // every header the tool sent except hop-by-hop / CDN / tracing ones, and
    // replace only the auth header inline with the real relay key.
    let request = forward_headers(state.client.post(&url), inbound_headers, upstream, protocol)
        .header(header::CONTENT_TYPE, "application/json")
        // Keep passthrough bodies inspectable and consistent with the local
        // response contract. The relay may otherwise honor the client's
        // `br` preference, while this gateway only guarantees identity bytes
        // to its downstream parser and SSE client.
        .header(header::ACCEPT_ENCODING, "identity")
        .body(body_bytes);

    let upstream_resp = match request.send().await {
        Ok(resp) => resp,
        Err(e) => {
            log::error!("Gateway passthrough to {} failed: {}", url, e);
            return error_response(StatusCode::BAD_GATEWAY, &format!("upstream error: {}", e));
        }
    };

    let status =
        StatusCode::from_u16(upstream_resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
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

    let upstream_content_type = upstream_resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("<missing>")
        .to_string();
    let upstream_content_encoding = upstream_resp
        .headers()
        .get(header::CONTENT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("identity")
        .to_string();

    log::info!(
        "Gateway passthrough upstream {} responded {}, stream={}, content_type={}, content_encoding={}",
        url,
        status.as_u16(),
        stream,
        upstream_content_type,
        upstream_content_encoding
    );

    if !stream {
        let bytes = match upstream_resp.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                log::error!(
                    "Gateway failed reading upstream JSON body from {}: {}",
                    url,
                    e
                );
                return error_response(
                    StatusCode::BAD_GATEWAY,
                    &format!("failed reading upstream body: {}", e),
                );
            }
        };
        let normalized = match extract_anthropic_message_document(&bytes) {
            Ok(value) => match serde_json::to_vec(&value) {
                Ok(bytes) => bytes,
                Err(e) => {
                    log::error!(
                        "Gateway failed serializing upstream JSON from {}: {}",
                        url,
                        e
                    );
                    return error_response(
                        StatusCode::BAD_GATEWAY,
                        "failed normalizing upstream JSON",
                    );
                }
            },
            Err(e) => {
                log::error!(
                    "Gateway got malformed JSON from {}: {} ({} bytes)",
                    url,
                    e,
                    bytes.len()
                );
                return error_response(StatusCode::BAD_GATEWAY, "upstream returned malformed JSON");
            }
        };

        return Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(normalized))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }

    // Stream Anthropic SSE incrementally, dropping relay-private billing
    // events that appear after message_stop and are not part of the Anthropic
    // protocol understood by Claude Code.
    let mut byte_stream = upstream_resp.bytes_stream();
    let filtered_stream = async_stream::stream! {
        let mut buffer = Vec::<u8>::new();
        while let Some(chunk) = byte_stream.next().await {
            match chunk {
                Ok(bytes) => buffer.extend_from_slice(&bytes),
                Err(error) => {
                    yield Err::<Vec<u8>, _>(std::io::Error::other(error.to_string()));
                    break;
                }
            }

            while let Some(end) = buffer.windows(2).position(|window| window == b"\n\n") {
                let block: Vec<u8> = buffer.drain(..end + 2).collect();
                if !is_relay_billing_sse_block(&block) {
                    yield Ok::<_, std::io::Error>(block);
                }
            }
        }
        if !buffer.is_empty() && !is_relay_billing_sse_block(&buffer) {
            yield Ok::<_, std::io::Error>(buffer);
        }
    };
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from_stream(filtered_stream))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn extract_anthropic_message_document(bytes: &[u8]) -> Result<Value, serde_json::Error> {
    match serde_json::from_slice::<Value>(bytes) {
        Ok(value) => return Ok(value),
        Err(primary_error) => {
            let mut fallback = None;
            let mut message = None;

            // Compatible relays sometimes prefix an otherwise valid JSON
            // document with a transport marker (`data:`, `0:`, etc.) or append
            // another private document. Probe each object boundary and keep
            // only a complete top-level value. This avoids logging or relying
            // on the response's user-visible text.
            for (offset, byte) in bytes.iter().enumerate() {
                if *byte != b'{' && *byte != b'[' {
                    continue;
                }
                let mut values =
                    serde_json::Deserializer::from_slice(&bytes[offset..]).into_iter::<Value>();
                let Some(Ok(value)) = values.next() else {
                    continue;
                };

                let message_like = value.get("type").and_then(Value::as_str) == Some("message")
                    || (value.get("role").and_then(Value::as_str) == Some("assistant")
                        && value.get("content").and_then(Value::as_array).is_some());
                if message_like {
                    message = Some(value);
                    break;
                }
                if value.get("error").is_some() {
                    fallback = Some(value);
                } else if fallback.is_none() && value.get("billing").is_none() {
                    fallback = Some(value);
                }
            }

            return message.or(fallback).ok_or(primary_error);
        }
    }
}

fn is_relay_billing_sse_block(block: &[u8]) -> bool {
    block
        .windows(b"event: billing_summary".len())
        .any(|window| window == b"event: billing_summary")
        || block
            .windows(b"\"type\":\"billing_summary\"".len())
            .any(|window| window == b"\"type\":\"billing_summary\"")
}

fn strip_one_m_model_marker(model: &str) -> &str {
    let trimmed = model.trim_end();
    let marker = "[1m]";
    if trimmed.len() >= marker.len()
        && trimmed[trimmed.len() - marker.len()..].eq_ignore_ascii_case(marker)
    {
        trimmed[..trimmed.len() - marker.len()].trim_end()
    } else {
        model
    }
}

/// Combine a relay base URL with an API endpoint: a base already ending in
/// `/v1` joins directly, a bare origin gains
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

/// Whether a header is hop-by-hop, CDN, or tracing metadata that must not be
/// forwarded to the upstream relay. Everything else (user-agent, originator,
/// session ids, and other client fingerprint headers a relay checks) is kept.
/// Uses a conservative proxy header skip list.
fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "host"
            | "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "content-length"
            | "x-forwarded-host"
            | "x-forwarded-for"
            | "x-forwarded-port"
            | "x-forwarded-proto"
            | "forwarded"
            | "cf-connecting-ip"
            | "cf-ipcountry"
            | "cf-ray"
            | "cf-visitor"
            | "true-client-ip"
            | "fastly-client-ip"
            | "x-request-id"
            | "x-correlation-id"
            | "x-trace-id"
            | "x-amzn-trace-id"
            | "x-b3-traceid"
            | "x-b3-spanid"
            | "x-b3-parentspanid"
            | "x-b3-sampled"
            | "traceparent"
            | "tracestate"
    )
}

/// Inject the real relay key in the shape the upstream protocol expects.
fn apply_auth(
    request: reqwest::RequestBuilder,
    upstream: &GatewayUpstream,
    inbound_header: Option<&str>,
) -> reqwest::RequestBuilder {
    match upstream.kind {
        // Claude Code configured with ANTHROPIC_AUTH_TOKEN authenticates with
        // `Authorization: Bearer ...`. Some relays (including agentrouter)
        // route bearer tokens and x-api-key credentials differently, so
        // changing the header shape can select the wrong channel even though
        // the credential itself is valid. Preserve the caller's auth shape and
        // only replace the secret value. Default to bearer because that is the
        // shape written by NeuraDock's Claude Code configuration.
        UpstreamKind::Anthropic if inbound_header == Some("x-api-key") => {
            request.header("x-api-key", &upstream.api_key)
        }
        UpstreamKind::Anthropic => request.bearer_auth(&upstream.api_key),
        UpstreamKind::OpenAiChat | UpstreamKind::OpenAiResponses => {
            request.bearer_auth(&upstream.api_key)
        }
    }
}

/// Canonical client fingerprint of a real Codex CLI, injected when
/// impersonation is on so any caller looks like Codex to the relay.
const CODEX_USER_AGENT: &str = "codex_cli_rs/0.20.0 (Mac OS 15.0.0; arm64) NeuraDock";
const CODEX_ORIGINATOR: &str = "codex_cli_rs";
/// Canonical client fingerprint of a real Claude Code CLI.
const CLAUDE_CODE_USER_AGENT: &str = "claude-cli/1.0.119 (external, cli)";

/// Whether a header is part of the client's identity fingerprint that a relay
/// inspects to recognise Codex / Claude Code. On the impersonation path these
/// are dropped from the caller and replaced with canonical values so an
/// arbitrary agent is presented as the expected client.
fn is_client_fingerprint_header(name: &str) -> bool {
    matches!(
        name,
        "user-agent"
            | "originator"
            | "session_id"
            | "session-id"
            | "thread-id"
            | "conversation_id"
            | "x-app"
            | "x-client-request-id"
            | "openai-beta"
    ) || name.starts_with("x-stainless-")
        || name.starts_with("x-codex-")
}

/// Copy the tool's request headers onto the upstream request. Relays may
/// fingerprint the client (user-agent, originator, session
/// id, ...) and reject requests that don't look like a real Codex / Claude
/// Code, so every header the tool sent is forwarded except hop-by-hop / CDN /
/// tracing ones. The auth header is replaced inline with the real relay key,
/// and Anthropic upstreams get a default `anthropic-version` when the client
/// did not send one. Used on both the passthrough and convert paths.
///
/// When `upstream.impersonate` is on, the caller's fingerprint headers are
/// dropped and replaced with a canonical Codex / Claude Code fingerprint, so a
/// non-Codex/Claude agent (or a bare script) is presented to the relay as the
/// expected client.
fn forward_headers(
    mut request: reqwest::RequestBuilder,
    inbound_headers: &axum::http::HeaderMap,
    upstream: &GatewayUpstream,
    protocol: InboundProtocol,
) -> reqwest::RequestBuilder {
    // Impersonation is for protocol conversion (for example
    // Codex -> Anthropic), not for an actual Claude Code request already
    // speaking Anthropic Messages. Replacing a real Claude client's
    // user-agent/x-stainless fingerprint can make strict relays choose a
    // different response path.
    let should_impersonate = upstream.impersonate
        && !matches!(
            (protocol, upstream.kind),
            (InboundProtocol::AnthropicMessages, UpstreamKind::Anthropic)
        );

    // Claude Code can carry both headers (for example when a shell-level
    // ANTHROPIC_API_KEY coexists with NeuraDock's ANTHROPIC_AUTH_TOKEN).
    // HeaderMap iteration order is not a protocol guarantee. Prefer bearer
    // auth explicitly because agentrouter routes it differently from
    // x-api-key; choosing whichever header happens to be visited first can
    // silently put the request into its `default` group.
    let inbound_auth_header = if inbound_headers.contains_key(header::AUTHORIZATION) {
        Some("authorization")
    } else if inbound_headers.contains_key("x-api-key") {
        Some("x-api-key")
    } else if inbound_headers.contains_key("x-goog-api-key") {
        Some("x-goog-api-key")
    } else {
        None
    };

    for (name, value) in inbound_headers {
        let key = name.as_str();

        if is_hop_by_hop_header(key) {
            continue;
        }

        if key.eq_ignore_ascii_case("authorization")
            || key.eq_ignore_ascii_case("x-api-key")
            || key.eq_ignore_ascii_case("x-goog-api-key")
        {
            continue;
        }

        // reqwest recomputes content-length from the body we set; the caller
        // sets content-type explicitly (the body may be reshaped), so skip
        // both here to avoid duplicate or mismatched headers.
        if key.eq_ignore_ascii_case("content-length") || key.eq_ignore_ascii_case("content-type") {
            continue;
        }

        // The gateway deliberately sets `Accept-Encoding: identity` for
        // Anthropic passthrough. Do not forward the client's gzip/br value as
        // a second header, otherwise the relay may still select Brotli and
        // the downstream JSON/SSE contract becomes undecodable.
        if key.eq_ignore_ascii_case("accept-encoding") {
            continue;
        }

        // On the impersonation path, drop the caller's own fingerprint headers;
        // canonical values are injected below.
        if should_impersonate && is_client_fingerprint_header(&key.to_ascii_lowercase()) {
            continue;
        }

        request = request.header(name.clone(), value.clone());
    }

    if should_impersonate {
        request = inject_canonical_fingerprint(request, inbound_headers, protocol);
    }

    request = apply_auth(request, upstream, inbound_auth_header);
    if upstream.kind == UpstreamKind::Anthropic
        && inbound_headers.get("anthropic-version").is_none()
    {
        request = request.header("anthropic-version", "2023-06-01");
    }
    request
}

/// Inject a canonical Codex / Claude Code fingerprint onto an impersonated
/// request. The tool a caller is bound to (by inbound protocol) decides which
/// identity to present. Session-style ids the caller already sent are reused;
/// otherwise a stable synthetic id is supplied so relays that require one are
/// satisfied. The value is derived from the caller's own session id when
/// present, so retries within a conversation stay consistent.
fn inject_canonical_fingerprint(
    request: reqwest::RequestBuilder,
    inbound_headers: &axum::http::HeaderMap,
    protocol: InboundProtocol,
) -> reqwest::RequestBuilder {
    match protocol {
        InboundProtocol::Responses => {
            let session = inbound_headers
                .get("session_id")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
                .unwrap_or_else(|| "neuradock-gateway-session".to_string());
            request
                .header(header::USER_AGENT, CODEX_USER_AGENT)
                .header("originator", CODEX_ORIGINATOR)
                .header("session_id", session)
                .header("openai-beta", "responses=experimental")
        }
        InboundProtocol::AnthropicMessages => request
            .header(header::USER_AGENT, CLAUDE_CODE_USER_AGENT)
            .header("x-app", "cli"),
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

    #[test]
    fn build_upstream_url_handles_versioned_and_custom_bases() {
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

    #[test]
    fn strips_claude_code_one_m_marker_before_upstream() {
        assert_eq!(
            strip_one_m_model_marker("claude-opus-4-8[1m]"),
            "claude-opus-4-8"
        );
        assert_eq!(
            strip_one_m_model_marker("claude-opus-4-8[1M]"),
            "claude-opus-4-8"
        );
        assert_eq!(
            strip_one_m_model_marker("claude-opus-4-8"),
            "claude-opus-4-8"
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
                impersonate: false,
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
                impersonate: false,
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
                (
                    [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
                    body,
                )
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
                impersonate: false,
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

    #[tokio::test]
    async fn passthrough_forwards_client_headers_and_replaces_auth() {
        // Relays fingerprint the client, so the tool's own headers (user-agent,
        // originator, session id) must reach the upstream; only auth is swapped
        // to the real key and tracing/hop-by-hop headers are dropped.
        use axum::http::HeaderMap;
        let seen = Arc::new(tokio::sync::Mutex::new(None::<HeaderMap>));
        let seen_clone = seen.clone();
        let app = Router::new().route(
            "/v1/responses",
            post(move |headers: HeaderMap| {
                let seen = seen_clone.clone();
                async move {
                    *seen.lock().await = Some(headers);
                    Json(json!({ "id": "ok" }))
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

        let mut map = HashMap::new();
        map.insert(
            GatewayClient::Codex,
            GatewayUpstream {
                base_url: format!("http://{}", addr),
                api_key: "sk-real-relay-key".to_string(),
                kind: UpstreamKind::OpenAiResponses,
                model: None,
                impersonate: false,
            },
        );
        let gateway = start_gateway(Arc::new(RwLock::new(map)), 0).await.unwrap();

        reqwest::Client::new()
            .post(format!("{}/responses", gateway.base_url()))
            .header("user-agent", "codex_cli_rs/1.0.0")
            .header("originator", "codex_cli_rs")
            .header("session_id", "abc-123")
            .header("authorization", "Bearer sk-placeholder")
            .header("x-amzn-trace-id", "should-be-dropped")
            .json(&json!({ "input": "ping" }))
            .send()
            .await
            .unwrap();

        let got = seen.lock().await.clone().unwrap();
        // Client fingerprint headers reach the upstream.
        assert_eq!(got.get("user-agent").unwrap(), "codex_cli_rs/1.0.0");
        assert_eq!(got.get("originator").unwrap(), "codex_cli_rs");
        assert_eq!(got.get("session_id").unwrap(), "abc-123");
        // Auth is replaced with the real relay key.
        assert_eq!(
            got.get("authorization").unwrap(),
            "Bearer sk-real-relay-key"
        );
        // Tracing header dropped.
        assert!(got.get("x-amzn-trace-id").is_none());

        gateway.shutdown().await;
    }

    #[tokio::test]
    async fn anthropic_passthrough_preserves_bearer_auth_shape() {
        // NeuraDock configures Claude Code with ANTHROPIC_AUTH_TOKEN, which
        // makes it send Authorization: Bearer. Relays may route that
        // differently from x-api-key, so the gateway must replace the token
        // without changing the header shape.
        use axum::http::HeaderMap;
        let seen = Arc::new(tokio::sync::Mutex::new(None::<HeaderMap>));
        let seen_clone = seen.clone();
        let app = Router::new().route(
            "/v1/messages",
            post(move |headers: HeaderMap| {
                let seen = seen_clone.clone();
                async move {
                    *seen.lock().await = Some(headers);
                    Json(json!({
                        "id": "msg_1",
                        "type": "message",
                        "role": "assistant",
                        "content": [{ "type": "text", "text": "pong" }],
                        "model": "claude-opus-4-8[1m]",
                        "stop_reason": "end_turn",
                        "usage": { "input_tokens": 1, "output_tokens": 1 }
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

        let mut map = HashMap::new();
        map.insert(
            GatewayClient::ClaudeCode,
            GatewayUpstream {
                base_url: format!("http://{}", addr),
                api_key: "relay-secret".to_string(),
                kind: UpstreamKind::Anthropic,
                model: Some("claude-opus-4-8[1m]".to_string()),
                impersonate: true,
            },
        );
        let gateway = start_gateway(Arc::new(RwLock::new(map)), 0).await.unwrap();

        let response: Value = reqwest::Client::new()
            .post(format!("{}/v1/messages", gateway.base_url()))
            .header("authorization", "Bearer local-placeholder")
            // A stale shell-level API key may coexist with AUTH_TOKEN. Bearer
            // must win regardless of HeaderMap iteration order.
            .header("x-api-key", "stale-local-api-key")
            .header("anthropic-version", "2023-06-01")
            .header("user-agent", "claude-cli/9.9.9 (external, cli)")
            .header("x-stainless-runtime", "node")
            .header(
                "anthropic-beta",
                "claude-code-20250219,context-1m-2025-08-07",
            )
            .json(&json!({
                "model": "claude-placeholder",
                "max_tokens": 16,
                "messages": [{ "role": "user", "content": "ping" }]
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        let got = seen.lock().await.clone().unwrap();
        assert_eq!(got.get("authorization").unwrap(), "Bearer relay-secret");
        assert!(got.get("x-api-key").is_none());
        assert_eq!(
            got.get("user-agent").unwrap(),
            "claude-cli/9.9.9 (external, cli)"
        );
        assert_eq!(got.get("x-stainless-runtime").unwrap(), "node");
        assert_eq!(
            got.get("anthropic-beta").unwrap(),
            "claude-code-20250219,context-1m-2025-08-07"
        );
        assert_eq!(response["model"], "claude-opus-4-8[1m]");

        gateway.shutdown().await;
    }

    #[tokio::test]
    async fn anthropic_non_stream_json_is_buffered_and_normalized() {
        // Some Anthropic-compatible relays return valid JSON as text/plain.
        // Claude Code expects a normal non-stream JSON response rather than a
        // chunked passthrough body with the relay's incorrect media type.
        let app = Router::new().route(
            "/v1/messages",
            post(|| async {
                (
                    [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                    format!(
                        "0:{}",
                        json!({
                            "billing": { "source": "request" },
                            "id": "msg_1",
                            "type": "message",
                            "role": "assistant",
                            "content": [{ "type": "text", "text": "pong" }],
                            "model": "claude-opus-4-8",
                            "stop_reason": "end_turn",
                            "usage": { "input_tokens": 1, "output_tokens": 1 }
                        })
                    ),
                )
            }),
        );
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let mut map = HashMap::new();
        map.insert(
            GatewayClient::ClaudeCode,
            GatewayUpstream {
                base_url: format!("http://{}", addr),
                api_key: "relay-secret".to_string(),
                kind: UpstreamKind::Anthropic,
                model: Some("claude-opus-4-8[1m]".to_string()),
                impersonate: false,
            },
        );
        let gateway = start_gateway(Arc::new(RwLock::new(map)), 0).await.unwrap();

        let response = reqwest::Client::new()
            .post(format!("{}/v1/messages", gateway.base_url()))
            .header("authorization", "Bearer placeholder")
            .json(&json!({
                "model": "claude-opus-4-8",
                "max_tokens": 16,
                "stream": false,
                "messages": [{ "role": "user", "content": "ping" }]
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("application/json")
        );
        assert!(response.content_length().is_some());
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["content"][0]["text"], "pong");

        gateway.shutdown().await;
    }

    #[tokio::test]
    async fn convert_path_also_forwards_client_headers() {
        // The chat-conversion path (chat-only relay) must forward the client's
        // fingerprint headers too, or the relay rejects it as an unknown client
        // (the agentrouter.org 401 regression).
        use axum::http::HeaderMap;
        let seen = Arc::new(tokio::sync::Mutex::new(None::<HeaderMap>));
        let seen_clone = seen.clone();
        let app = Router::new().route(
            "/v1/chat/completions",
            post(move |headers: HeaderMap| {
                let seen = seen_clone.clone();
                async move {
                    *seen.lock().await = Some(headers);
                    Json(json!({
                        "choices": [{ "finish_reason": "stop", "message": { "content": "ok" } }]
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

        let mut map = HashMap::new();
        map.insert(
            GatewayClient::Codex,
            GatewayUpstream {
                base_url: format!("http://{}", addr),
                api_key: "sk-real-relay-key".to_string(),
                kind: UpstreamKind::OpenAiChat, // chat-only relay -> convert path
                model: None,
                impersonate: false,
            },
        );
        let gateway = start_gateway(Arc::new(RwLock::new(map)), 0).await.unwrap();

        reqwest::Client::new()
            .post(format!("{}/responses", gateway.base_url()))
            .header("user-agent", "codex_cli_rs/1.0.0")
            .header("originator", "codex_cli_rs")
            .header("authorization", "Bearer sk-placeholder")
            .json(&json!({ "input": "ping" }))
            .send()
            .await
            .unwrap();

        let got = seen.lock().await.clone().unwrap();
        assert_eq!(got.get("user-agent").unwrap(), "codex_cli_rs/1.0.0");
        assert_eq!(got.get("originator").unwrap(), "codex_cli_rs");
        assert_eq!(
            got.get("authorization").unwrap(),
            "Bearer sk-real-relay-key"
        );

        gateway.shutdown().await;
    }

    #[tokio::test]
    async fn impersonation_rewrites_caller_fingerprint_to_codex() {
        // A non-Codex agent (e.g. a bare script with its own user-agent) must be
        // presented to the relay as a canonical Codex client when impersonation
        // is on.
        use axum::http::HeaderMap;
        let seen = Arc::new(tokio::sync::Mutex::new(None::<HeaderMap>));
        let seen_clone = seen.clone();
        let app = Router::new().route(
            "/v1/responses",
            post(move |headers: HeaderMap| {
                let seen = seen_clone.clone();
                async move {
                    *seen.lock().await = Some(headers);
                    Json(json!({ "id": "ok" }))
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

        let mut map = HashMap::new();
        map.insert(
            GatewayClient::Codex,
            GatewayUpstream {
                base_url: format!("http://{}", addr),
                api_key: "sk-real".to_string(),
                kind: UpstreamKind::OpenAiResponses,
                model: None,
                impersonate: true,
            },
        );
        let gateway = start_gateway(Arc::new(RwLock::new(map)), 0).await.unwrap();

        reqwest::Client::new()
            .post(format!("{}/responses", gateway.base_url()))
            .header("user-agent", "my-random-script/9.9")
            .header("originator", "not-codex")
            .json(&json!({ "input": "ping" }))
            .send()
            .await
            .unwrap();

        let got = seen.lock().await.clone().unwrap();
        // The caller's own fingerprint is replaced with a canonical Codex one.
        assert!(got
            .get("user-agent")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("codex_cli_rs/"));
        assert_eq!(got.get("originator").unwrap(), "codex_cli_rs");
        assert!(got.get("session_id").is_some());
        assert!(got.get("openai-beta").is_some());

        gateway.shutdown().await;
    }
}
