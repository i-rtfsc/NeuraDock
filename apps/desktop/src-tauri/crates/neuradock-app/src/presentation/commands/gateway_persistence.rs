use serde::{Deserialize, Serialize};

use super::gateway::{GatewayClientDto, GatewaySourceDto, UpstreamKindDto};
use super::gateway_state::GatewayState;
use crate::presentation::error::CommandError;
use neuradock_infrastructure::fs_safe::atomic_write;
use tauri::Manager;

pub(super) const BINDINGS_FILENAME: &str = "gateway_bindings.json";
pub(super) const RUNTIME_FILENAME: &str = "gateway_runtime.json";

#[derive(Debug, Clone)]
pub(super) struct BindingMeta {
    pub(super) label: String,
    pub(super) source: GatewaySourceDto,
    pub(super) kind: UpstreamKindDto,
    pub(super) model: Option<String>,
    pub(super) context1m: bool,
    pub(super) impersonate: bool,
    pub(super) isolated: Option<IsolatedTarget>,
    pub(super) enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct IsolatedTarget {
    pub(super) slug: String,
    pub(super) display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct PersistedBinding {
    pub(super) client: GatewayClientDto,
    pub(super) source: GatewaySourceDto,
    pub(super) kind: UpstreamKindDto,
    pub(super) model: Option<String>,
    #[serde(default)]
    pub(super) context1m: bool,
    #[serde(default)]
    pub(super) impersonate: bool,
    pub(super) label: String,
    #[serde(default)]
    pub(super) isolated: Option<IsolatedTarget>,
    #[serde(default = "default_true")]
    pub(super) enabled: bool,
}

fn default_true() -> bool {
    true
}

pub(super) async fn write_bindings(
    app: &tauri::AppHandle,
    state: &GatewayState,
) -> Result<(), CommandError> {
    let dir = app.path().app_data_dir().map_err(|e| {
        CommandError::infrastructure(format!("Failed to get app data directory: {e}"))
    })?;
    let meta = state.meta.lock().await;
    let bindings: Vec<PersistedBinding> = meta
        .iter()
        .map(|(client, meta)| PersistedBinding {
            client: client.client.into(),
            source: meta.source.clone(),
            kind: meta.kind,
            model: meta.model.clone(),
            context1m: meta.context1m,
            impersonate: meta.impersonate,
            label: meta.label.clone(),
            isolated: meta.isolated.clone(),
            enabled: meta.enabled,
        })
        .collect();
    let json = serde_json::to_string_pretty(&bindings)
        .map_err(|e| CommandError::infrastructure(e.to_string()))?;
    atomic_write(&dir.join(BINDINGS_FILENAME), &json, false)
        .map_err(|e| CommandError::infrastructure(e.to_string()))
}
