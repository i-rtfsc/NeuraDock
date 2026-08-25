use super::server::{GatewayUpstream, UpstreamKind};
use super::transform::InboundProtocol;
use axum::http::{header, HeaderMap};
use reqwest::RequestBuilder;

pub(crate) const CODEX_USER_AGENT: &str = "codex_cli_rs/0.20.0 (Mac OS 15.0.0; arm64) NeuraDock";
pub(crate) const CODEX_ORIGINATOR: &str = "codex_cli_rs";
pub(crate) const CLAUDE_CODE_USER_AGENT: &str = "claude-cli/1.0.119 (external, cli)";

pub(crate) fn is_hop_by_hop_header(name: &str) -> bool {
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

pub(crate) fn is_client_fingerprint_header(name: &str) -> bool {
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

pub(crate) fn inject_canonical_fingerprint(
    request: RequestBuilder,
    inbound_headers: &HeaderMap,
    protocol: InboundProtocol,
) -> RequestBuilder {
    match protocol {
        InboundProtocol::Responses => {
            let session = inbound_headers
                .get("session_id")
                .and_then(|value| value.to_str().ok())
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

pub(crate) fn apply_auth(
    request: RequestBuilder,
    upstream: &GatewayUpstream,
    inbound_header: Option<&str>,
) -> RequestBuilder {
    match upstream.kind {
        UpstreamKind::Anthropic if inbound_header == Some("x-api-key") => {
            request.header("x-api-key", &upstream.api_key)
        }
        UpstreamKind::Anthropic => request.bearer_auth(&upstream.api_key),
        UpstreamKind::OpenAiChat | UpstreamKind::OpenAiResponses => {
            request.bearer_auth(&upstream.api_key)
        }
    }
}
