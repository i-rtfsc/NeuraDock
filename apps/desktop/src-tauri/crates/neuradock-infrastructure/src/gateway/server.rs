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
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use futures::StreamExt;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::net::TcpListener;
use uuid::Uuid;

use super::reliability::{usage_from_json, GatewayRuntime, GatewayTokenUsage};
use super::stream::StreamBridge;
use super::transform::{
    anthropic_to_chat, chat_to_anthropic, chat_to_responses, requested_model, responses_to_chat,
    wants_stream, InboundProtocol,
};

const MAX_INBOUND_BODY_BYTES: usize = 64 * 1024 * 1024;
const MAX_NON_STREAM_BODY_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_BODY_BYTES: usize = 1024 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const RESPONSE_HEADER_TIMEOUT: Duration = Duration::from_secs(120);
const NON_STREAM_BODY_TIMEOUT: Duration = Duration::from_secs(600);
const STREAM_FIRST_EVENT_TIMEOUT: Duration = Duration::from_secs(90);
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);

/// A tool the gateway can serve. Requests are routed to a client by URL path
/// (`/responses` -> Codex, `/v1/messages` -> Claude Code), and each client is
/// independently bound to its own upstream relay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayClient {
    /// Codex CLI, speaking the OpenAI Responses protocol.
    Codex,
    /// Claude Code, speaking the Anthropic Messages protocol.
    ClaudeCode,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GatewayRoute {
    pub client: GatewayClient,
    pub profile: String,
}

impl GatewayRoute {
    pub fn default(client: GatewayClient) -> Self {
        Self {
            client,
            profile: "default".to_string(),
        }
    }
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
    /// Stable source reference used for health and usage aggregation.
    pub id: String,
    /// Human-readable source label; never contains key material.
    pub label: String,
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
pub type GatewayBindings = Arc<RwLock<HashMap<GatewayRoute, GatewayUpstream>>>;

struct GatewayState {
    bindings: GatewayBindings,
    client: reqwest::Client,
    runtime: Arc<GatewayRuntime>,
    local_credential: Arc<str>,
}

/// A running gateway bound to a localhost port.
pub struct GatewayHandle {
    addr: SocketAddr,
    local_credential: Arc<str>,
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

    /// Random credential required for calls to this gateway process.
    pub fn local_credential(&self) -> &str {
        &self.local_credential
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
    start_gateway_with_runtime(bindings, GatewayRuntime::shared(), port).await
}

pub async fn start_gateway_with_runtime(
    bindings: GatewayBindings,
    runtime: Arc<GatewayRuntime>,
    port: u16,
) -> Result<GatewayHandle, std::io::Error> {
    let local_credential: Arc<str> = generate_local_credential().into();
    let state = Arc::new(GatewayState {
        bindings,
        client: reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(30))
            .build()
            .map_err(std::io::Error::other)?,
        runtime,
        local_credential: local_credential.clone(),
    });

    let app = Router::new()
        .route("/responses", post(handle_responses))
        .route("/v1/responses", post(handle_responses))
        .route("/v1/messages", post(handle_anthropic))
        .route(
            "/profiles/:profile/v1/responses",
            post(handle_profile_responses),
        )
        .route(
            "/profiles/:profile/v1/messages",
            post(handle_profile_anthropic),
        )
        .layer(DefaultBodyLimit::max(MAX_INBOUND_BODY_BYTES))
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
        local_credential,
        shutdown: Some(tx),
        join,
    })
}

fn generate_local_credential() -> String {
    let mut random = [0_u8; 32];
    OsRng.fill_bytes(&mut random);
    // Must start with "sk-" so client config writers (ensure_sk_prefix)
    // store it verbatim; any rewrite would break the exact-match check.
    let mut credential = String::with_capacity(71);
    credential.push_str("sk-ndg-");
    for byte in random {
        use std::fmt::Write as _;
        let _ = write!(credential, "{byte:02x}");
    }
    credential
}

async fn handle_responses(
    State(state): State<Arc<GatewayState>>,
    headers: axum::http::HeaderMap,
    body: Result<axum::body::Bytes, axum::extract::rejection::BytesRejection>,
) -> Response {
    let request_id = Uuid::new_v4().to_string();
    let body = match body {
        Ok(body) => body,
        Err(_) => {
            return error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "request body exceeded the gateway limit",
                &request_id,
            )
        }
    };
    handle(
        state,
        GatewayRoute::default(GatewayClient::Codex),
        headers,
        body,
        request_id,
    )
    .await
}

async fn handle_profile_responses(
    Path(profile): Path<String>,
    State(state): State<Arc<GatewayState>>,
    headers: axum::http::HeaderMap,
    body: Result<axum::body::Bytes, axum::extract::rejection::BytesRejection>,
) -> Response {
    let request_id = Uuid::new_v4().to_string();
    match body {
        Ok(body) => {
            handle(
                state,
                GatewayRoute {
                    client: GatewayClient::Codex,
                    profile,
                },
                headers,
                body,
                request_id,
            )
            .await
        }
        Err(_) => error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request body exceeded the gateway limit",
            &request_id,
        ),
    }
}

async fn handle_anthropic(
    State(state): State<Arc<GatewayState>>,
    headers: axum::http::HeaderMap,
    body: Result<axum::body::Bytes, axum::extract::rejection::BytesRejection>,
) -> Response {
    let request_id = Uuid::new_v4().to_string();
    let body = match body {
        Ok(body) => body,
        Err(_) => {
            return error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "request body exceeded the gateway limit",
                &request_id,
            )
        }
    };
    handle(
        state,
        GatewayRoute::default(GatewayClient::ClaudeCode),
        headers,
        body,
        request_id,
    )
    .await
}

async fn handle_profile_anthropic(
    Path(profile): Path<String>,
    State(state): State<Arc<GatewayState>>,
    headers: axum::http::HeaderMap,
    body: Result<axum::body::Bytes, axum::extract::rejection::BytesRejection>,
) -> Response {
    let request_id = Uuid::new_v4().to_string();
    match body {
        Ok(body) => {
            handle(
                state,
                GatewayRoute {
                    client: GatewayClient::ClaudeCode,
                    profile,
                },
                headers,
                body,
                request_id,
            )
            .await
        }
        Err(_) => error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request body exceeded the gateway limit",
            &request_id,
        ),
    }
}

struct SentUpstream {
    response: reqwest::Response,
    half_open_permit: bool,
    started_at: Instant,
    permit_guard: GatewayPermitGuard,
}

struct GatewayPermitGuard {
    runtime: Arc<GatewayRuntime>,
    client: GatewayClient,
    upstream_id: String,
    armed: bool,
}

impl GatewayPermitGuard {
    fn new(
        runtime: Arc<GatewayRuntime>,
        client: GatewayClient,
        upstream_id: String,
        armed: bool,
    ) -> Self {
        Self {
            runtime,
            client,
            upstream_id,
            armed,
        }
    }

    fn complete(&mut self) {
        self.armed = false;
    }
}

impl Drop for GatewayPermitGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let runtime = self.runtime.clone();
        let client = self.client;
        let upstream_id = self.upstream_id.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                runtime.release_neutral(client, &upstream_id).await;
            });
        }
    }
}

async fn send_to_upstream<F>(
    state: &GatewayState,
    client: GatewayClient,
    upstream: &GatewayUpstream,
    request_id: &str,
    build_request: F,
) -> Result<SentUpstream, Response>
where
    F: Fn(&GatewayUpstream) -> reqwest::RequestBuilder,
{
    let Some(candidate) = state.runtime.candidate(client, upstream).await else {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "the configured upstream is temporarily unavailable",
            request_id,
        ));
    };

    let started_at = Instant::now();
    let send = build_request(&candidate.upstream).send();
    match tokio::time::timeout(RESPONSE_HEADER_TIMEOUT, send).await {
        Err(_) => {
            let message = format!(
                "upstream response header timed out after {} seconds",
                RESPONSE_HEADER_TIMEOUT.as_secs()
            );
            state
                .runtime
                .record_failure(
                    client,
                    &candidate.upstream,
                    candidate.half_open_permit,
                    started_at.elapsed(),
                    true,
                    message.clone(),
                )
                .await;
            Err(error_response(
                StatusCode::GATEWAY_TIMEOUT,
                &message,
                request_id,
            ))
        }
        Ok(Ok(response)) => Ok(SentUpstream {
            response,
            half_open_permit: candidate.half_open_permit,
            started_at,
            permit_guard: GatewayPermitGuard::new(
                state.runtime.clone(),
                client,
                candidate.upstream.id.clone(),
                candidate.half_open_permit,
            ),
        }),
        Ok(Err(error)) => {
            let message = transport_error_message(&error);
            state
                .runtime
                .record_failure(
                    client,
                    &candidate.upstream,
                    candidate.half_open_permit,
                    started_at.elapsed(),
                    true,
                    message.clone(),
                )
                .await;
            Err(error_response(
                StatusCode::BAD_GATEWAY,
                &format!("upstream error: {message}"),
                request_id,
            ))
        }
    }
}

async fn handle(
    state: Arc<GatewayState>,
    route: GatewayRoute,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
    request_id: String,
) -> Response {
    let client = route.client;
    let protocol = client.protocol();

    if !has_valid_local_credential(&headers, &state.local_credential) {
        log::warn!(
            "Rejected unauthorized local gateway request for {} (request_id={})",
            client.display_name(),
            request_id
        );
        return error_response(
            StatusCode::UNAUTHORIZED,
            "invalid local gateway credential",
            &request_id,
        );
    }

    // Look up this client's upstream binding; each client is bound
    // independently and requests for unbound clients are rejected.
    let bindings = state.bindings.read().await;
    let Some(upstream_source) = bindings.get(&route).cloned() else {
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
            &request_id,
        );
    };
    drop(bindings);
    // Passthrough when the upstream already speaks the tool's protocol
    // using native protocol passthrough: forward with the real key, no
    // translation, streaming bytes proxied as-is.
    let passthrough = matches!(
        (protocol, upstream_source.kind),
        (InboundProtocol::Responses, UpstreamKind::OpenAiResponses)
            | (InboundProtocol::AnthropicMessages, UpstreamKind::Anthropic)
    );
    if passthrough {
        return forward_passthrough(
            &state,
            client,
            protocol,
            &upstream_source,
            &headers,
            body,
            &request_id,
        )
        .await;
    }

    // Cross-protocol combinations other than "-> chat" are not implemented yet.
    if upstream_source.kind != UpstreamKind::OpenAiChat {
        return error_response(
            StatusCode::NOT_IMPLEMENTED,
            &format!(
                "{} with a {:?} upstream is not supported yet; bind a chat or native upstream",
                client.display_name(),
                upstream_source.kind
            ),
            &request_id,
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
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("invalid JSON: {}", e),
                &request_id,
            );
        }
    };
    if let Err(message) = validate_conversion_request(protocol, &inbound) {
        return error_response(StatusCode::UNPROCESSABLE_ENTITY, &message, &request_id);
    }

    let stream = wants_stream(&inbound);
    let model = upstream_source
        .model
        .clone()
        .or_else(|| requested_model(&inbound))
        .unwrap_or_default();

    // Translate inbound -> Chat Completions.
    let model_override = upstream_source.model.as_deref();
    let mut chat_body = match protocol {
        InboundProtocol::Responses => responses_to_chat(&inbound, model_override),
        InboundProtocol::AnthropicMessages => anthropic_to_chat(&inbound, model_override),
    };
    // Force the stream flag to match what the tool asked for.
    chat_body["stream"] = Value::Bool(stream);
    if stream {
        chat_body["stream_options"] = serde_json::json!({ "include_usage": true });
    }

    let url = build_upstream_url(&upstream_source.base_url, "/chat/completions");
    let display_url = sanitized_url_for_log(&url);
    log::info!(
        "Gateway {} -> {} (convert, model={}, stream={})",
        client.display_name(),
        display_url,
        model,
        stream
    );
    // Forward the tool's headers (user-agent, originator, session id, ...) so
    // the relay's client fingerprint check passes, replacing only auth. The
    // body is our translated chat body, so drop the client's content-length.
    let chat_bytes = serde_json::to_vec(&chat_body).unwrap_or_default();
    let sent = match send_to_upstream(&state, client, &upstream_source, &request_id, |candidate| {
        let candidate_url = build_upstream_url(&candidate.base_url, "/chat/completions");
        forward_headers(
            state.client.post(candidate_url),
            &headers,
            candidate,
            protocol,
        )
        .header(header::CONTENT_TYPE, "application/json")
        .body(chat_bytes.clone())
    })
    .await
    {
        Ok(sent) => sent,
        Err(response) => return response,
    };
    let half_open_permit = sent.half_open_permit;
    let started_at = sent.started_at;
    let upstream = sent.response;
    let mut permit_guard = sent.permit_guard;

    if !upstream.status().is_success() {
        let status =
            StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
        let text = read_response_text(upstream, MAX_ERROR_BODY_BYTES, NON_STREAM_BODY_TIMEOUT)
            .await
            .unwrap_or_default();
        let message = redact_sensitive_message(
            &safe_upstream_message(status, &text),
            &upstream_source.api_key,
        );
        log::error!(
            "Gateway upstream {} returned {} ({})",
            display_url,
            status.as_u16(),
            message
        );
        state
            .runtime
            .record_failure(
                client,
                &upstream_source,
                half_open_permit,
                started_at.elapsed(),
                circuit_failure_status(status),
                format!("HTTP {}", status.as_u16()),
            )
            .await;
        permit_guard.complete();
        return error_response(status, &message, &request_id);
    }

    log::info!(
        "Gateway upstream {} responded OK, translating (stream={})",
        display_url,
        stream
    );
    if stream {
        stream_response(
            protocol,
            model,
            upstream,
            state.runtime.clone(),
            client,
            upstream_source,
            half_open_permit,
            started_at,
            permit_guard,
            request_id,
        )
    } else {
        match read_response_json(upstream, MAX_NON_STREAM_BODY_BYTES, NON_STREAM_BODY_TIMEOUT).await
        {
            Ok(chat) => {
                if let Err(message) = validate_chat_response(&chat) {
                    let message = redact_sensitive_message(&message, &upstream_source.api_key);
                    state
                        .runtime
                        .record_failure(
                            client,
                            &upstream_source,
                            half_open_permit,
                            started_at.elapsed(),
                            true,
                            message.clone(),
                        )
                        .await;
                    permit_guard.complete();
                    return error_response(StatusCode::BAD_GATEWAY, &message, &request_id);
                }
                state
                    .runtime
                    .record_success(
                        client,
                        &upstream_source,
                        half_open_permit,
                        usage_from_json(&chat),
                        started_at.elapsed(),
                        None,
                    )
                    .await;
                permit_guard.complete();
                let translated = match protocol {
                    InboundProtocol::Responses => chat_to_responses(&chat, &model),
                    InboundProtocol::AnthropicMessages => chat_to_anthropic(&chat, &model),
                };
                json_response(StatusCode::OK, &translated)
            }
            Err(e) => {
                log::error!(
                    "Gateway got an invalid upstream body from {}: {}",
                    display_url,
                    e
                );
                state
                    .runtime
                    .record_failure(
                        client,
                        &upstream_source,
                        half_open_permit,
                        started_at.elapsed(),
                        true,
                        e.clone(),
                    )
                    .await;
                permit_guard.complete();
                error_response(StatusCode::BAD_GATEWAY, &e, &request_id)
            }
        }
    }
}

fn has_valid_local_credential(headers: &axum::http::HeaderMap, expected: &str) -> bool {
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    bearer == Some(expected) || api_key == Some(expected)
}

/// Forward a request verbatim to a same-protocol upstream: real key injected,
/// optional model override, status and body (including SSE bytes) proxied
/// through unchanged. This is the default path for native upstreams, matching
/// a standard non-converting forward.
async fn forward_passthrough(
    state: &GatewayState,
    client: GatewayClient,
    protocol: InboundProtocol,
    upstream_source: &GatewayUpstream,
    inbound_headers: &axum::http::HeaderMap,
    body: axum::body::Bytes,
    request_id: &str,
) -> Response {
    let endpoint = match protocol {
        InboundProtocol::Responses => "/responses",
        InboundProtocol::AnthropicMessages => "/v1/messages",
    };
    let url = build_upstream_url(&upstream_source.base_url, endpoint);
    let display_url = sanitized_url_for_log(&url);
    log::info!(
        "Gateway {} -> {} (passthrough, model_override={:?})",
        client.display_name(),
        display_url,
        upstream_source.model
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
    let body_bytes = if let Some(model) = &upstream_source.model {
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
    let sent = match send_to_upstream(state, client, upstream_source, request_id, |candidate| {
        let candidate_url = build_upstream_url(&candidate.base_url, endpoint);
        forward_headers(
            state.client.post(candidate_url),
            inbound_headers,
            candidate,
            protocol,
        )
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCEPT_ENCODING, "identity")
        .body(body_bytes.clone())
    })
    .await
    {
        Ok(sent) => sent,
        Err(response) => return response,
    };
    let upstream_source = upstream_source.clone();
    let half_open_permit = sent.half_open_permit;
    let started_at = sent.started_at;
    let upstream_resp = sent.response;
    let mut permit_guard = sent.permit_guard;

    let status =
        StatusCode::from_u16(upstream_resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    if !status.is_success() {
        let text = read_response_text(upstream_resp, MAX_ERROR_BODY_BYTES, NON_STREAM_BODY_TIMEOUT)
            .await
            .unwrap_or_default();
        let message = redact_sensitive_message(
            &safe_upstream_message(status, &text),
            &upstream_source.api_key,
        );
        log::error!(
            "Gateway passthrough upstream {} returned {} ({})",
            display_url,
            status.as_u16(),
            message
        );
        state
            .runtime
            .record_failure(
                client,
                &upstream_source,
                half_open_permit,
                started_at.elapsed(),
                circuit_failure_status(status),
                format!("HTTP {}", status.as_u16()),
            )
            .await;
        permit_guard.complete();
        return error_response(status, &message, request_id);
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
        display_url,
        status.as_u16(),
        stream,
        upstream_content_type,
        upstream_content_encoding
    );

    if !stream {
        let bytes = match read_response_bytes(
            upstream_resp,
            MAX_NON_STREAM_BODY_BYTES,
            NON_STREAM_BODY_TIMEOUT,
        )
        .await
        {
            Ok(bytes) => bytes,
            Err(e) => {
                log::error!(
                    "Gateway failed reading upstream JSON body from {}: {}",
                    display_url,
                    e
                );
                state
                    .runtime
                    .record_failure(
                        client,
                        &upstream_source,
                        half_open_permit,
                        started_at.elapsed(),
                        true,
                        "failed reading upstream body".to_string(),
                    )
                    .await;
                permit_guard.complete();
                return error_response(
                    StatusCode::BAD_GATEWAY,
                    &format!("failed reading upstream body: {}", e),
                    request_id,
                );
            }
        };
        let (normalized_value, normalized) = match extract_anthropic_message_document(&bytes) {
            Ok(value) => match serde_json::to_vec(&value) {
                Ok(bytes) => (value, bytes),
                Err(e) => {
                    log::error!(
                        "Gateway failed serializing upstream JSON from {}: {}",
                        display_url,
                        e
                    );
                    state
                        .runtime
                        .record_failure(
                            client,
                            &upstream_source,
                            half_open_permit,
                            started_at.elapsed(),
                            true,
                            "failed normalizing upstream JSON".to_string(),
                        )
                        .await;
                    permit_guard.complete();
                    return error_response(
                        StatusCode::BAD_GATEWAY,
                        "failed normalizing upstream JSON",
                        request_id,
                    );
                }
            },
            Err(e) => {
                log::error!(
                    "Gateway got malformed JSON from {}: {} ({} bytes)",
                    display_url,
                    e,
                    bytes.len()
                );
                state
                    .runtime
                    .record_failure(
                        client,
                        &upstream_source,
                        half_open_permit,
                        started_at.elapsed(),
                        true,
                        "upstream returned malformed JSON".to_string(),
                    )
                    .await;
                permit_guard.complete();
                return error_response(
                    StatusCode::BAD_GATEWAY,
                    "upstream returned malformed JSON",
                    request_id,
                );
            }
        };

        if let Err(message) = validate_native_response(protocol, &normalized_value) {
            let message = redact_sensitive_message(&message, &upstream_source.api_key);
            state
                .runtime
                .record_failure(
                    client,
                    &upstream_source,
                    half_open_permit,
                    started_at.elapsed(),
                    true,
                    message.clone(),
                )
                .await;
            permit_guard.complete();
            return error_response(StatusCode::BAD_GATEWAY, &message, request_id);
        }

        state
            .runtime
            .record_success(
                client,
                &upstream_source,
                half_open_permit,
                usage_from_json(&normalized_value),
                started_at.elapsed(),
                None,
            )
            .await;
        permit_guard.complete();

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
    let runtime = state.runtime.clone();
    let usage_upstream = upstream_source.clone();
    let request_id = request_id.to_string();
    let filtered_stream = async_stream::stream! {
        const MAX_PENDING_SSE_BYTES: usize = 4 * 1024 * 1024;
        let mut buffer = Vec::<u8>::new();
        let mut usage = GatewayTokenUsage::default();
        let mut valid_event_seen = false;
        let mut terminal_seen = false;
        let mut first_event_latency = None;
        let first_event_deadline = tokio::time::Instant::now() + STREAM_FIRST_EVENT_TIMEOUT;

        loop {
            let wait = if valid_event_seen {
                STREAM_IDLE_TIMEOUT
            } else {
                first_event_deadline.saturating_duration_since(tokio::time::Instant::now())
            };
            if wait.is_zero() {
                let message = format!(
                    "upstream stream produced no valid event within {} seconds",
                    STREAM_FIRST_EVENT_TIMEOUT.as_secs()
                );
                runtime
                    .record_failure(
                        client,
                        &usage_upstream,
                        half_open_permit,
                        started_at.elapsed(),
                        true,
                        message.clone(),
                    )
                    .await;
                permit_guard.complete();
                yield Ok::<_, std::io::Error>(native_stream_error(protocol, &message, &request_id));
                return;
            }

            let next = match tokio::time::timeout(wait, byte_stream.next()).await {
                Ok(next) => next,
                Err(_) => {
                    let seconds = if valid_event_seen {
                        STREAM_IDLE_TIMEOUT.as_secs()
                    } else {
                        STREAM_FIRST_EVENT_TIMEOUT.as_secs()
                    };
                    let message = format!("upstream stream timed out after {seconds} seconds");
                    runtime
                        .record_failure(
                            client,
                            &usage_upstream,
                            half_open_permit,
                            started_at.elapsed(),
                            true,
                            message.clone(),
                        )
                        .await;
                    permit_guard.complete();
                    yield Ok::<_, std::io::Error>(native_stream_error(protocol, &message, &request_id));
                    return;
                }
            };

            let Some(chunk) = next else { break };
            let bytes = match chunk {
                Ok(bytes) => bytes,
                Err(_) => {
                    let message = "upstream stream transport failed".to_string();
                    runtime
                        .record_failure(
                            client,
                            &usage_upstream,
                            half_open_permit,
                            started_at.elapsed(),
                            true,
                            message.clone(),
                        )
                        .await;
                    permit_guard.complete();
                    yield Ok::<_, std::io::Error>(native_stream_error(protocol, &message, &request_id));
                    return;
                }
            };
            if buffer.len().saturating_add(bytes.len()) > MAX_PENDING_SSE_BYTES {
                let message = "upstream SSE event exceeded the buffer limit".to_string();
                runtime
                    .record_failure(
                        client,
                        &usage_upstream,
                        half_open_permit,
                        started_at.elapsed(),
                        true,
                        message.clone(),
                    )
                    .await;
                permit_guard.complete();
                yield Ok::<_, std::io::Error>(native_stream_error(protocol, &message, &request_id));
                return;
            }
            buffer.extend_from_slice(&bytes);

            while let Some(block) = take_sse_block(&mut buffer) {
                usage.merge(usage_from_sse_block(&block));
                match inspect_native_sse_block(protocol, &block) {
                    Ok(observation) => {
                        if observation.valid && !valid_event_seen {
                            first_event_latency = Some(started_at.elapsed());
                        }
                        valid_event_seen |= observation.valid;
                        terminal_seen |= observation.terminal;
                    }
                    Err(message) => {
                        let message =
                            redact_sensitive_message(&message, &usage_upstream.api_key);
                        runtime
                            .record_failure(
                                client,
                                &usage_upstream,
                                half_open_permit,
                                started_at.elapsed(),
                                true,
                                message.clone(),
                            )
                            .await;
                        permit_guard.complete();
                        yield Ok::<_, std::io::Error>(native_stream_error(
                            protocol,
                            &message,
                            &request_id,
                        ));
                        return;
                    }
                }
                if !is_relay_billing_sse_block(&block) {
                    yield Ok::<_, std::io::Error>(block);
                }
            }
        }

        if !valid_event_seen || !terminal_seen {
            let message = if !valid_event_seen {
                "upstream stream ended without a valid protocol event"
            } else {
                "upstream stream ended before its terminal event"
            }
            .to_string();
            runtime
                .record_failure(
                    client,
                    &usage_upstream,
                    half_open_permit,
                    started_at.elapsed(),
                    true,
                    message.clone(),
                )
                .await;
            permit_guard.complete();
            yield Ok::<_, std::io::Error>(native_stream_error(protocol, &message, &request_id));
            return;
        }

        runtime
            .record_success(
                client,
                &usage_upstream,
                half_open_permit,
                usage,
                started_at.elapsed(),
                first_event_latency,
            )
            .await;
        permit_guard.complete();
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
        Ok(value) => Ok(value),
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

            message.or(fallback).ok_or(primary_error)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeSseObservation {
    valid: bool,
    terminal: bool,
}

fn take_sse_block(buffer: &mut Vec<u8>) -> Option<Vec<u8>> {
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

fn inspect_native_sse_block(
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
    let payload_type = value.get("type").and_then(Value::as_str);
    let event_type = event_name.or(payload_type);

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
                return Err(value_error_message(&value)
                    .unwrap_or_else(|| "upstream Responses stream failed".to_string()));
            }
            Ok(NativeSseObservation { valid, terminal })
        }
    }
}

fn native_stream_error(protocol: InboundProtocol, message: &str, request_id: &str) -> Vec<u8> {
    let message = sanitize_error_message(message);
    match protocol {
        InboundProtocol::AnthropicMessages => format!(
            "event: error\ndata: {}\n\n",
            serde_json::json!({
                "type": "error",
                "error": {
                    "type": "api_error",
                    "message": message,
                },
                "request_id": request_id,
            })
        )
        .into_bytes(),
        InboundProtocol::Responses => format!(
            "event: response.failed\ndata: {}\n\n",
            serde_json::json!({
                "type": "response.failed",
                "response": {
                    "id": format!("resp_{}", request_id.replace('-', "")),
                    "object": "response",
                    "status": "failed",
                    "output": [],
                    "error": {
                        "type": "upstream_error",
                        "message": message,
                        "request_id": request_id,
                    },
                },
            })
        )
        .into_bytes(),
    }
}

fn usage_from_sse_block(block: &[u8]) -> GatewayTokenUsage {
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

/// True when `status` should trip the circuit breaker.
///
/// 401 and 403 are excluded: they reveal that the configured key is wrong or
/// revoked, which is user-correctable state rather than upstream instability.
/// Counting them toward the failure threshold meant that after 3 honest auth
/// errors the breaker opened and started returning 503 for 60 seconds, so the
/// user never saw the 401 that would have told them to fix their key.
fn circuit_failure_status(status: StatusCode) -> bool {
    status.is_server_error()
        || matches!(
            status,
            StatusCode::REQUEST_TIMEOUT | StatusCode::TOO_MANY_REQUESTS
        )
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

fn sanitized_url_for_log(value: &str) -> String {
    let Ok(mut parsed) = url::Url::parse(value) else {
        return "<invalid-upstream-url>".to_string();
    };
    let _ = parsed.set_username("");
    let _ = parsed.set_password(None);
    parsed.set_query(None);
    parsed.set_fragment(None);
    parsed.to_string()
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
    runtime: Arc<GatewayRuntime>,
    client: GatewayClient,
    usage_upstream: GatewayUpstream,
    half_open_permit: bool,
    started_at: Instant,
    mut permit_guard: GatewayPermitGuard,
    request_id: String,
) -> Response {
    let mut byte_stream = upstream.bytes_stream();

    let sse = async_stream::stream! {
        const MAX_PENDING_SSE_BYTES: usize = 4 * 1024 * 1024;
        let mut buffer = Vec::<u8>::new();
        let mut bridge = StreamBridge::new(protocol, model);
        let mut terminal_seen = false;
        let mut semantic_event_seen = false;
        let mut first_event_latency = None;
        let first_event_deadline = tokio::time::Instant::now() + STREAM_FIRST_EVENT_TIMEOUT;

        'upstream: loop {
            let wait = if semantic_event_seen {
                STREAM_IDLE_TIMEOUT
            } else {
                first_event_deadline.saturating_duration_since(tokio::time::Instant::now())
            };
            if wait.is_zero() {
                let message = format!(
                    "upstream stream produced no valid event within {} seconds",
                    STREAM_FIRST_EVENT_TIMEOUT.as_secs()
                );
                runtime
                    .record_failure(
                        client,
                        &usage_upstream,
                        half_open_permit,
                        started_at.elapsed(),
                        true,
                        message.clone(),
                    )
                    .await;
                permit_guard.complete();
                for event in bridge.failure_events(&message, &request_id) {
                    yield Ok::<_, std::io::Error>(event.to_wire().into_bytes());
                }
                return;
            }

            let next = match tokio::time::timeout(wait, byte_stream.next()).await {
                Ok(next) => next,
                Err(_) => {
                    let seconds = if semantic_event_seen {
                        STREAM_IDLE_TIMEOUT.as_secs()
                    } else {
                        STREAM_FIRST_EVENT_TIMEOUT.as_secs()
                    };
                    let message = format!("upstream stream timed out after {seconds} seconds");
                    runtime
                        .record_failure(
                            client,
                            &usage_upstream,
                            half_open_permit,
                            started_at.elapsed(),
                            true,
                            message.clone(),
                        )
                        .await;
                    permit_guard.complete();
                    for event in bridge.failure_events(&message, &request_id) {
                        yield Ok::<_, std::io::Error>(event.to_wire().into_bytes());
                    }
                    return;
                }
            };

            let Some(chunk) = next else { break };
            let bytes = match chunk {
                Ok(bytes) => bytes,
                Err(_) => {
                    let message = "upstream stream transport failed".to_string();
                    runtime
                        .record_failure(
                            client,
                            &usage_upstream,
                            half_open_permit,
                            started_at.elapsed(),
                            true,
                            message.clone(),
                        )
                        .await;
                    permit_guard.complete();
                    for event in bridge.failure_events(&message, &request_id) {
                        yield Ok::<_, std::io::Error>(event.to_wire().into_bytes());
                    }
                    return;
                }
            };
            if buffer.len().saturating_add(bytes.len()) > MAX_PENDING_SSE_BYTES {
                let message = "upstream SSE event exceeded the buffer limit".to_string();
                runtime
                    .record_failure(
                        client,
                        &usage_upstream,
                        half_open_permit,
                        started_at.elapsed(),
                        true,
                        message.clone(),
                    )
                    .await;
                permit_guard.complete();
                for event in bridge.failure_events(&message, &request_id) {
                    yield Ok::<_, std::io::Error>(event.to_wire().into_bytes());
                }
                return;
            }
            buffer.extend_from_slice(&bytes);

            while let Some(newline) = buffer.iter().position(|byte| *byte == b'\n') {
                let mut line: Vec<u8> = buffer.drain(..=newline).collect();
                while matches!(line.last(), Some(b'\n' | b'\r')) {
                    line.pop();
                }
                let Ok(line) = std::str::from_utf8(&line) else {
                    let message = "upstream SSE contained invalid UTF-8".to_string();
                    runtime
                        .record_failure(
                            client,
                            &usage_upstream,
                            half_open_permit,
                            started_at.elapsed(),
                            true,
                            message.clone(),
                        )
                        .await;
                    permit_guard.complete();
                    for event in bridge.failure_events(&message, &request_id) {
                        yield Ok::<_, std::io::Error>(event.to_wire().into_bytes());
                    }
                    return;
                };
                let Some(payload) = line.trim().strip_prefix("data:") else {
                    continue;
                };
                let payload = payload.trim();
                if payload.is_empty() {
                    continue;
                }
                if payload == "[DONE]" {
                    terminal_seen = true;
                    break 'upstream;
                }
                let chat_chunk = match serde_json::from_str::<Value>(payload) {
                    Ok(value) => value,
                    Err(error) => {
                        let message = format!("upstream SSE contained malformed JSON: {error}");
                        runtime
                            .record_failure(
                                client,
                                &usage_upstream,
                                half_open_permit,
                                started_at.elapsed(),
                                true,
                                message.clone(),
                            )
                            .await;
                        permit_guard.complete();
                        for event in bridge.failure_events(&message, &request_id) {
                            yield Ok::<_, std::io::Error>(event.to_wire().into_bytes());
                        }
                        return;
                    }
                };
                let events = bridge.observe(&chat_chunk);
                if !events.is_empty() {
                    if !semantic_event_seen {
                        first_event_latency = Some(started_at.elapsed());
                    }
                    semantic_event_seen = true;
                }
                for event in events {
                    yield Ok::<_, std::io::Error>(event.to_wire().into_bytes());
                }
            }
        }

        match bridge.finish(terminal_seen) {
            Ok(events) => {
                runtime
                    .record_success(
                        client,
                        &usage_upstream,
                        half_open_permit,
                        bridge.usage(),
                        started_at.elapsed(),
                        first_event_latency,
                    )
                    .await;
                permit_guard.complete();
                for event in events {
                    yield Ok::<_, std::io::Error>(event.to_wire().into_bytes());
                }
            }
            Err(message) => {
                runtime
                    .record_failure(
                        client,
                        &usage_upstream,
                        half_open_permit,
                        started_at.elapsed(),
                        true,
                        message.clone(),
                    )
                    .await;
                permit_guard.complete();
                for event in bridge.failure_events(&message, &request_id) {
                    yield Ok::<_, std::io::Error>(event.to_wire().into_bytes());
                }
            }
        }
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from_stream(sse))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

async fn read_response_bytes(
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

async fn read_response_text(
    response: reqwest::Response,
    max_bytes: usize,
    timeout: Duration,
) -> Result<String, String> {
    let bytes = read_response_bytes(response, max_bytes, timeout).await?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

async fn read_response_json(
    response: reqwest::Response,
    max_bytes: usize,
    timeout: Duration,
) -> Result<Value, String> {
    let bytes = read_response_bytes(response, max_bytes, timeout).await?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("upstream returned malformed JSON: {error}"))
}

fn value_error_message(value: &Value) -> Option<String> {
    value
        .pointer("/error/message")
        .or_else(|| value.get("message"))
        .and_then(Value::as_str)
        .map(sanitize_error_message)
}

fn sanitize_error_message(message: &str) -> String {
    let compact = message.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(500).collect()
}

fn redact_sensitive_message(message: &str, secret: &str) -> String {
    let sanitized = sanitize_error_message(message);
    if secret.is_empty() {
        sanitized
    } else {
        sanitized.replace(secret, "[redacted]")
    }
}

fn transport_error_message(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "upstream request timed out".to_string()
    } else if error.is_connect() {
        "upstream connection failed".to_string()
    } else if error.is_request() {
        "upstream request could not be sent".to_string()
    } else {
        "upstream transport failed".to_string()
    }
}

fn safe_upstream_message(status: StatusCode, body: &str) -> String {
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        if let Some(message) = value_error_message(&value) {
            if !message.is_empty() {
                return message;
            }
        }
    }
    format!("upstream returned HTTP {}", status.as_u16())
}

fn validate_chat_response(value: &Value) -> Result<(), String> {
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

fn validate_conversion_request(protocol: InboundProtocol, value: &Value) -> Result<(), String> {
    // cache_control is intentionally NOT rejected. Claude Code attaches it
    // unconditionally when prompt caching is on, so rejecting it made the
    // whole Anthropic -> Chat path unusable. It is also not silently lost:
    // the Chat body is rebuilt field-by-field from an allowlist
    // (`anthropic_to_chat`), so the key cannot reach the upstream. Dropping a
    // cache hint only costs money, never correctness -- unlike the content
    // types below, which change what the model actually sees.

    // Tool types are not validated here: the conversion layer emulates
    // tool_search through a proxy function and drops hosted tools it cannot
    // express in Chat Completions (web_search, file_search, ...).

    let unsupported_content_types: &[&str] = match protocol {
        InboundProtocol::Responses => &["input_image", "input_file", "computer_screenshot"],
        InboundProtocol::AnthropicMessages => {
            &["image", "document", "thinking", "redacted_thinking"]
        }
    };
    if let Some(content_type) = find_content_type(value, unsupported_content_types) {
        return Err(format!(
            "content type {content_type} is not supported through Chat protocol conversion"
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
        return Err(
            "encrypted reasoning without a summary cannot be preserved through Chat protocol conversion"
                .to_string(),
        );
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

fn validate_native_response(protocol: InboundProtocol, value: &Value) -> Result<(), String> {
    if value.get("error").is_some() {
        return Err(value_error_message(value)
            .unwrap_or_else(|| "upstream returned an error object".to_string()));
    }

    match protocol {
        InboundProtocol::AnthropicMessages => {
            if value.get("type").and_then(Value::as_str) != Some("message")
                || !value.get("content").is_some_and(Value::is_array)
            {
                return Err("upstream returned an invalid Anthropic message".to_string());
            }
        }
        InboundProtocol::Responses => {
            if value.get("status").and_then(Value::as_str) == Some("failed") {
                return Err(value_error_message(value)
                    .unwrap_or_else(|| "upstream Responses request failed".to_string()));
            }
            if value.get("object").and_then(Value::as_str) != Some("response")
                || !value.get("output").is_some_and(Value::is_array)
            {
                return Err("upstream returned an invalid Responses object".to_string());
            }
        }
    }
    Ok(())
}

fn json_response(status: StatusCode, value: &Value) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/json")],
        value.to_string(),
    )
        .into_response()
}

fn error_response(status: StatusCode, message: &str, request_id: &str) -> Response {
    let error_type = match status {
        StatusCode::BAD_REQUEST => "invalid_request_error",
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => "authentication_error",
        StatusCode::PAYLOAD_TOO_LARGE => "request_too_large",
        StatusCode::UNPROCESSABLE_ENTITY | StatusCode::NOT_IMPLEMENTED => "unsupported_request",
        StatusCode::TOO_MANY_REQUESTS => "rate_limit_error",
        StatusCode::BAD_GATEWAY => "upstream_error",
        StatusCode::SERVICE_UNAVAILABLE => "service_unavailable",
        StatusCode::GATEWAY_TIMEOUT => "timeout_error",
        _ if status.is_server_error() => "upstream_error",
        _ => "gateway_error",
    };
    let body = serde_json::json!({
        "error": {
            "message": sanitize_error_message(message),
            "type": error_type,
            "request_id": request_id,
        }
    });
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-request-id", request_id)
        .body(Body::from(body.to_string()))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::post, Json, Router};
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

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
    fn log_url_removes_credentials_query_and_fragment() {
        let display = sanitized_url_for_log(
            "https://user:secret@relay.example/v1/responses?token=secret#debug",
        );
        assert_eq!(display, "https://relay.example/v1/responses");
        assert!(!display.contains("secret"));
    }

    #[test]
    fn local_gateway_credentials_are_random_and_high_entropy() {
        let first = generate_local_credential();
        let second = generate_local_credential();
        assert!(first.starts_with("sk-ndg-"));
        assert_eq!(first.len(), 71);
        assert_ne!(first, second);
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

    #[test]
    fn parses_usage_from_native_sse_events() {
        let responses = usage_from_sse_block(
            br#"event: response.completed
data: {"response":{"usage":{"input_tokens":12,"output_tokens":5}}}

"#,
        );
        assert_eq!(responses.input_tokens, 12);
        assert_eq!(responses.output_tokens, 5);

        let anthropic = usage_from_sse_block(
            br#"event: message_start
data: {"message":{"usage":{"input_tokens":9,"cache_read_input_tokens":3}}}

"#,
        );
        assert_eq!(anthropic.input_tokens, 9);
        assert_eq!(anthropic.cache_read_tokens, 3);
    }

    #[test]
    fn splits_lf_and_crlf_sse_blocks_without_losing_bytes() {
        let mut buffer = b"event: one\r\ndata: {}\r\n\r\nevent: two\ndata: {}\n\nrest".to_vec();
        assert_eq!(
            take_sse_block(&mut buffer).unwrap(),
            b"event: one\r\ndata: {}\r\n\r\n"
        );
        assert_eq!(
            take_sse_block(&mut buffer).unwrap(),
            b"event: two\ndata: {}\n\n"
        );
        assert_eq!(buffer, b"rest");
    }

    #[test]
    fn inspects_native_stream_lifecycle_and_rejects_error_events() {
        let started = inspect_native_sse_block(
            InboundProtocol::AnthropicMessages,
            b"event: message_start\ndata: {\"type\":\"message_start\"}\n\n",
        )
        .unwrap();
        assert!(started.valid);
        assert!(!started.terminal);

        let completed = inspect_native_sse_block(
            InboundProtocol::Responses,
            b"event: response.completed\r\ndata: {\"type\":\"response.completed\"}\r\n\r\n",
        )
        .unwrap();
        assert!(completed.valid);
        assert!(completed.terminal);

        let error = inspect_native_sse_block(
            InboundProtocol::AnthropicMessages,
            b"event: error\ndata: {\"type\":\"error\",\"error\":{\"message\":\"bad upstream\"}}\n\n",
        )
        .unwrap_err();
        assert_eq!(error, "bad upstream");
    }

    #[test]
    fn conversion_rejects_fields_that_cannot_be_preserved() {
        let image = json!({
            "input": [{
                "role": "user",
                "content": [{ "type": "input_image", "image_url": "data:image/png;base64,AA==" }]
            }]
        });
        assert!(
            validate_conversion_request(InboundProtocol::Responses, &image)
                .unwrap_err()
                .contains("input_image")
        );

        let cache_control = json!({
            "messages": [{
                "role": "user",
                "content": [{
                    "type": "text",
                    "text": "hello",
                    "cache_control": { "type": "ephemeral" }
                }]
            }]
        });
        // cache_control must be ACCEPTED. Claude Code sends it on every request
        // when prompt caching is enabled, so rejecting it made the flagship
        // "drive Claude Code through a Chat relay" path fail on the first call.
        // It is dropped during conversion rather than refused.
        assert!(
            validate_conversion_request(InboundProtocol::AnthropicMessages, &cache_control).is_ok()
        );
    }

    #[test]
    fn cache_control_is_stripped_before_reaching_the_upstream() {
        // Guards the reason it is safe to accept: the Chat body is rebuilt from
        // an allowlist, so the key cannot leak upstream even though we no
        // longer reject it.
        let anthropic = json!({
            "model": "claude-3-5-sonnet",
            "system": [{
                "type": "text",
                "text": "be brief",
                "cache_control": { "type": "ephemeral" }
            }],
            "messages": [{
                "role": "user",
                "content": [{
                    "type": "text",
                    "text": "hello",
                    "cache_control": { "type": "ephemeral" }
                }]
            }],
            "max_tokens": 16
        });

        let converted = crate::gateway::transform::anthropic_to_chat(&anthropic, None);
        let rendered = serde_json::to_string(&converted).expect("serialize converted body");
        assert!(
            !rendered.contains("cache_control"),
            "cache_control leaked into the outbound Chat body: {rendered}"
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
            GatewayRoute::default(client),
            test_upstream(relay_url, UpstreamKind::OpenAiChat, Some(model), false),
        );
        Arc::new(RwLock::new(map))
    }

    fn test_upstream(
        base_url: String,
        kind: UpstreamKind,
        model: Option<&str>,
        impersonate: bool,
    ) -> GatewayUpstream {
        test_upstream_with_key(base_url, "sk-test", kind, model, impersonate)
    }

    fn test_upstream_with_key(
        base_url: String,
        api_key: &str,
        kind: UpstreamKind,
        model: Option<&str>,
        impersonate: bool,
    ) -> GatewayUpstream {
        GatewayUpstream {
            id: format!("test:{base_url}"),
            label: "Test relay".to_string(),
            base_url,
            api_key: api_key.to_string(),
            kind,
            model: model.map(str::to_string),
            impersonate,
        }
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
            .bearer_auth(gateway.local_credential())
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
            .bearer_auth(gateway.local_credential())
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
            .bearer_auth(gateway.local_credential())
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
    async fn local_gateway_rejects_missing_and_wrong_credentials() {
        let (relay_url, _seen) = spawn_fake_relay().await;
        let gateway = start_gateway(
            bindings_for(GatewayClient::Codex, relay_url, "relay-model"),
            0,
        )
        .await
        .unwrap();
        let client = reqwest::Client::new();

        let missing = client
            .post(format!("{}/responses", gateway.base_url()))
            .json(&json!({ "input": "ping" }))
            .send()
            .await
            .unwrap();
        assert_eq!(missing.status(), reqwest::StatusCode::UNAUTHORIZED);

        let wrong = client
            .post(format!("{}/responses", gateway.base_url()))
            .bearer_auth("wrong-local-credential")
            .json(&json!({ "input": "ping" }))
            .send()
            .await
            .unwrap();
        assert_eq!(wrong.status(), reqwest::StatusCode::UNAUTHORIZED);

        let accepted = client
            .post(format!("{}/responses", gateway.base_url()))
            .header("x-api-key", gateway.local_credential())
            .json(&json!({ "input": "ping" }))
            .send()
            .await
            .unwrap();
        assert_eq!(accepted.status(), reqwest::StatusCode::OK);

        gateway.shutdown().await;
    }

    #[tokio::test]
    async fn bindings_can_change_while_running() {
        let (relay_url, seen) = spawn_fake_relay().await;
        let bindings = bindings_for(GatewayClient::Codex, relay_url.clone(), "first");
        let gateway = start_gateway(bindings.clone(), 0).await.unwrap();

        // Rebind Codex to a different model without restarting.
        bindings.write().await.insert(
            GatewayRoute::default(GatewayClient::Codex),
            test_upstream(relay_url, UpstreamKind::OpenAiChat, Some("second"), false),
        );

        let client = reqwest::Client::new();
        client
            .post(format!("{}/responses", gateway.base_url()))
            .bearer_auth(gateway.local_credential())
            .json(&json!({ "input": "ping" }))
            .send()
            .await
            .unwrap();

        let upstream_body = seen.lock().await.clone().unwrap();
        assert_eq!(upstream_body["model"], "second");

        gateway.shutdown().await;
    }

    #[tokio::test]
    async fn connection_failure_returns_without_replaying() {
        let unavailable_listener =
            std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let unavailable_addr = unavailable_listener.local_addr().unwrap();
        drop(unavailable_listener);

        let upstream = test_upstream(
            format!("http://{unavailable_addr}"),
            UpstreamKind::OpenAiChat,
            Some("model"),
            false,
        );
        let bindings = Arc::new(RwLock::new(HashMap::from([(
            GatewayRoute::default(GatewayClient::Codex),
            upstream,
        )])));
        let gateway = start_gateway(bindings, 0).await.unwrap();

        let response = reqwest::Client::new()
            .post(format!("{}/responses", gateway.base_url()))
            .bearer_auth(gateway.local_credential())
            .json(&json!({ "input": "ping" }))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::BAD_GATEWAY);
        gateway.shutdown().await;
    }

    #[tokio::test]
    async fn http_error_does_not_replay_current_request() {
        let primary_requests = Arc::new(AtomicUsize::new(0));
        let primary_requests_clone = primary_requests.clone();
        let primary_app = Router::new().route(
            "/v1/chat/completions",
            post(move || {
                let primary_requests = primary_requests_clone.clone();
                async move {
                    primary_requests.fetch_add(1, Ordering::SeqCst);
                    (StatusCode::INTERNAL_SERVER_ERROR, "primary failed")
                }
            }),
        );
        let primary_listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let primary_addr = primary_listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(primary_listener, primary_app).await.unwrap();
        });

        let upstream = test_upstream(
            format!("http://{primary_addr}"),
            UpstreamKind::OpenAiChat,
            Some("model"),
            false,
        );
        let bindings = Arc::new(RwLock::new(HashMap::from([(
            GatewayRoute::default(GatewayClient::Codex),
            upstream,
        )])));
        let gateway = start_gateway(bindings, 0).await.unwrap();

        let response = reqwest::Client::new()
            .post(format!("{}/responses", gateway.base_url()))
            .bearer_auth(gateway.local_credential())
            .json(&json!({ "input": "ping" }))
            .send()
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            reqwest::StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(primary_requests.load(Ordering::SeqCst), 1);
        gateway.shutdown().await;
    }

    #[tokio::test]
    async fn open_circuit_blocks_later_requests() {
        let primary_requests = Arc::new(AtomicUsize::new(0));
        let primary_requests_clone = primary_requests.clone();
        let primary_app = Router::new().route(
            "/v1/chat/completions",
            post(move || {
                let primary_requests = primary_requests_clone.clone();
                async move {
                    primary_requests.fetch_add(1, Ordering::SeqCst);
                    (StatusCode::INTERNAL_SERVER_ERROR, "primary failed")
                }
            }),
        );
        let primary_listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let primary_addr = primary_listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(primary_listener, primary_app).await.unwrap();
        });

        let upstream = test_upstream(
            format!("http://{primary_addr}"),
            UpstreamKind::OpenAiChat,
            Some("model"),
            false,
        );
        let bindings = Arc::new(RwLock::new(HashMap::from([(
            GatewayRoute::default(GatewayClient::Codex),
            upstream,
        )])));
        let gateway = start_gateway(bindings, 0).await.unwrap();
        let client = reqwest::Client::new();

        const FAILURE_LIMIT: usize = 3;
        for _ in 0..FAILURE_LIMIT {
            let response = client
                .post(format!("{}/responses", gateway.base_url()))
                .bearer_auth(gateway.local_credential())
                .json(&json!({ "input": "ping" }))
                .send()
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                reqwest::StatusCode::INTERNAL_SERVER_ERROR
            );
        }

        let response = client
            .post(format!("{}/responses", gateway.base_url()))
            .bearer_auth(gateway.local_credential())
            .json(&json!({ "input": "ping" }))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(primary_requests.load(Ordering::SeqCst), FAILURE_LIMIT);
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
            .bearer_auth(gateway.local_credential())
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
        assert!(sse.contains("pong"), "unexpected stream: {sse}");

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
            .bearer_auth(gateway.local_credential())
            .json(&json!({ "messages": [{ "role": "user", "content": "ping" }], "stream": true }))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        assert!(sse.contains("event: message_start"));
        assert!(sse.contains("event: message_stop"));
        assert!(sse.contains("\"text\":\"po\""), "unexpected stream: {sse}");
        assert!(sse.contains("\"text\":\"ng\""), "unexpected stream: {sse}");

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
                    Json(json!({
                        "id": "resp_upstream",
                        "object": "response",
                        "status": "completed",
                        "output": [],
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
        let relay_url = format!("http://{}", addr);

        let mut map = HashMap::new();
        map.insert(
            GatewayRoute::default(GatewayClient::Codex),
            test_upstream(
                relay_url,
                UpstreamKind::OpenAiResponses,
                Some("relay-model"),
                false,
            ),
        );
        let gateway = start_gateway(Arc::new(RwLock::new(map)), 0).await.unwrap();

        let client = reqwest::Client::new();
        let resp: Value = client
            .post(format!("{}/responses", gateway.base_url()))
            .bearer_auth(gateway.local_credential())
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
            GatewayRoute::default(GatewayClient::Codex),
            test_upstream_with_key(
                format!("http://{}", addr),
                "sk-real-relay-key",
                UpstreamKind::OpenAiResponses,
                None,
                false,
            ),
        );
        let gateway = start_gateway(Arc::new(RwLock::new(map)), 0).await.unwrap();

        reqwest::Client::new()
            .post(format!("{}/responses", gateway.base_url()))
            .bearer_auth(gateway.local_credential())
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
            GatewayRoute::default(GatewayClient::ClaudeCode),
            test_upstream_with_key(
                format!("http://{}", addr),
                "relay-secret",
                UpstreamKind::Anthropic,
                Some("claude-opus-4-8[1m]"),
                true,
            ),
        );
        let gateway = start_gateway(Arc::new(RwLock::new(map)), 0).await.unwrap();

        let response: Value = reqwest::Client::new()
            .post(format!("{}/v1/messages", gateway.base_url()))
            .bearer_auth(gateway.local_credential())
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
            GatewayRoute::default(GatewayClient::ClaudeCode),
            test_upstream(
                format!("http://{}", addr),
                UpstreamKind::Anthropic,
                Some("claude-opus-4-8[1m]"),
                false,
            ),
        );
        let gateway = start_gateway(Arc::new(RwLock::new(map)), 0).await.unwrap();

        let response = reqwest::Client::new()
            .post(format!("{}/v1/messages", gateway.base_url()))
            .bearer_auth(gateway.local_credential())
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
            GatewayRoute::default(GatewayClient::Codex),
            test_upstream_with_key(
                format!("http://{}", addr),
                "sk-real-relay-key",
                UpstreamKind::OpenAiChat,
                None,
                false,
            ),
        );
        let gateway = start_gateway(Arc::new(RwLock::new(map)), 0).await.unwrap();

        reqwest::Client::new()
            .post(format!("{}/responses", gateway.base_url()))
            .bearer_auth(gateway.local_credential())
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
            GatewayRoute::default(GatewayClient::Codex),
            test_upstream(
                format!("http://{}", addr),
                UpstreamKind::OpenAiResponses,
                None,
                true,
            ),
        );
        let gateway = start_gateway(Arc::new(RwLock::new(map)), 0).await.unwrap();

        reqwest::Client::new()
            .post(format!("{}/responses", gateway.base_url()))
            .bearer_auth(gateway.local_credential())
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
