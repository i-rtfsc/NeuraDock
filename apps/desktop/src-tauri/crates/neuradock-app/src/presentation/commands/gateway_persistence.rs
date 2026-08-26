use serde::{Deserialize, Serialize};

use super::agent_access::{AgentAccessDto, AgentAccessState, AgentIdDto};
use super::gateway::{GatewayClientDto, GatewaySourceDto, UpstreamKindDto};
use super::gateway_state::GatewayState;
use crate::presentation::error::CommandError;
use neuradock_infrastructure::fs_safe::atomic_write;
use neuradock_infrastructure::gateway::{GatewayRoute, GatewayUpstream};
use std::path::PathBuf;
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

pub(super) fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, CommandError> {
    app.path()
        .app_data_dir()
        .map_err(|e| CommandError::infrastructure(format!("Failed to get app data directory: {e}")))
}

pub(super) fn read_bindings(dir: &std::path::Path) -> Result<Vec<PersistedBinding>, CommandError> {
    let content = std::fs::read_to_string(dir.join(BINDINGS_FILENAME))
        .map_err(|e| CommandError::infrastructure(e.to_string()))?;
    serde_json::from_str(&content).map_err(|e| {
        CommandError::infrastructure(format!("Failed to parse saved gateway bindings: {e}"))
    })
}

pub(super) async fn restore_binding_memory(
    state: &GatewayState,
    route: GatewayRoute,
    upstream: Option<GatewayUpstream>,
    meta: Option<BindingMeta>,
) {
    let mut bindings = state.bindings.write().await;
    match upstream {
        Some(value) => {
            bindings.insert(route.clone(), value);
        }
        None => {
            bindings.remove(&route);
        }
    }
    drop(bindings);
    let mut metadata = state.meta.lock().await;
    match meta {
        Some(value) => {
            metadata.insert(route, value);
        }
        None => {
            metadata.remove(&route);
        }
    }
}

pub(super) fn restore_access_record(
    app: &tauri::AppHandle,
    access: &AgentAccessState,
    agent: AgentIdDto,
    record: Option<AgentAccessDto>,
) -> Result<(), CommandError> {
    match record {
        Some(value) => access.set(app, value),
        None => access.remove(app, agent),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn rollback_binding_change(
    app: &tauri::AppHandle,
    state: &GatewayState,
    access: &AgentAccessState,
    route: GatewayRoute,
    agent: AgentIdDto,
    old_upstream: Option<GatewayUpstream>,
    old_meta: Option<BindingMeta>,
    old_access: Option<AgentAccessDto>,
) {
    restore_binding_memory(state, route, old_upstream, old_meta).await;
    if let Err(error) = write_bindings(app, state).await {
        log::error!("Failed to roll back gateway bindings: {}", error);
    }
    if let Err(error) = restore_access_record(app, access, agent, old_access) {
        log::error!("Failed to roll back agent access record: {}", error);
    }
}
