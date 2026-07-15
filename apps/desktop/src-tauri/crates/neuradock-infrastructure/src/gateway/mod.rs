//! Embedded localhost provider-adaptation gateway.
//!
//! See [`server`] for the HTTP layer and [`transform`] / [`stream`] for the
//! protocol translation. The gateway adapts a Chat-Completions-only relay to
//! the OpenAI Responses protocol (Codex) or the Anthropic Messages protocol
//! (Claude Code, e.g. when backed by DeepSeek).

pub mod server;
pub mod stream;
pub mod transform;

pub use server::{start_gateway, GatewayHandle, GatewayProfile};
pub use transform::InboundProtocol;
