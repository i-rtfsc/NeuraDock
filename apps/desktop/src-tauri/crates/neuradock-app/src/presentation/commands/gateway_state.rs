use super::gateway_persistence::BindingMeta;
use neuradock_infrastructure::gateway::{
    GatewayBindings, GatewayClient, GatewayHandle, GatewayRoute, GatewayRuntime,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

pub struct GatewayState {
    pub(super) bindings: GatewayBindings,
    pub(super) meta: Mutex<HashMap<GatewayRoute, BindingMeta>>,
    pub(super) server: Mutex<Option<GatewayHandle>>,
    pub(super) runtime: Arc<GatewayRuntime>,
    pub(super) codex_operation: Mutex<()>,
    pub(super) claude_operation: Mutex<()>,
}

impl Default for GatewayState {
    fn default() -> Self {
        Self {
            bindings: Arc::new(RwLock::new(HashMap::new())),
            meta: Mutex::new(HashMap::new()),
            server: Mutex::new(None),
            runtime: GatewayRuntime::shared(),
            codex_operation: Mutex::new(()),
            claude_operation: Mutex::new(()),
        }
    }
}

impl GatewayState {
    pub(super) fn operation_lock(&self, client: GatewayClient) -> &Mutex<()> {
        match client {
            GatewayClient::Codex => &self.codex_operation,
            GatewayClient::ClaudeCode => &self.claude_operation,
        }
    }
}
