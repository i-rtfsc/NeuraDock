//! Embedded localhost provider-adaptation gateway.
//!
//! See [`server`] for the HTTP layer and [`transform`] / [`stream`] for the
//! protocol translation. The gateway adapts a Chat-Completions-only relay to
//! the OpenAI Responses protocol (Codex) or the Anthropic Messages protocol
//! (Claude Code, e.g. when backed by DeepSeek).

pub(crate) mod headers;
pub mod reliability;
pub(crate) mod response;
pub(crate) mod sanitization;
pub mod server;
pub mod stream;
pub mod transform;
pub(crate) mod validation;

pub use reliability::{
    usage_from_json, CircuitState, GatewayRuntime, GatewayTokenUsage, GatewayUpstreamSnapshot,
};
pub use server::{
    start_gateway, start_gateway_with_runtime, GatewayBindings, GatewayClient, GatewayHandle,
    GatewayRoute, GatewayUpstream, UpstreamKind,
};
pub use transform::InboundProtocol;
