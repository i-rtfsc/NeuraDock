use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Manager, State};
use tokio::sync::{Mutex, RwLock};

use crate::presentation::error::CommandError;
use crate::presentation::state::{Repositories, Services};
use neuradock_domain::independent_key::IndependentKeyId;
use neuradock_domain::shared::{AccountId, ProviderId};
use neuradock_infrastructure::gateway::{
    start_gateway, GatewayBindings, GatewayClient, GatewayHandle, GatewayUpstream, UpstreamKind,
};

use crate::application::services::GatewayTakeover;

/// Runtime state for the local adaptation gateway: one server, one port,
/// path-routed clients, each independently bound to an upstream relay key.
pub struct GatewayState {
    bindings: GatewayBindings,
    meta: Mutex<HashMap<GatewayClient, BindingMeta>>,
    server: Mutex<Option<GatewayHandle>>,
}

impl Default for GatewayState {
    fn default() -> Self {
        Self {
            bindings: Arc::new(RwLock::new(HashMap::new())),
            meta: Mutex::new(HashMap::new()),
            server: Mutex::new(None),
        }
    }
}

/// Per-client runtime metadata plus everything needed to persist and rebuild
/// the binding across restarts (the real key is never stored; it is re-resolved
/// from the source).
#[derive(Debug, Clone)]
struct BindingMeta {
    label: String,
    source: GatewaySourceDto,
    kind: UpstreamKindDto,
    model: Option<String>,
    impersonate: bool,
}

/// On-disk shape of a saved binding. Keys are never written; only the reference
/// (account token ids or independent key id) is, so the key is re-resolved on
/// startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedBinding {
    client: GatewayClientDto,
    source: GatewaySourceDto,
    kind: UpstreamKindDto,
    model: Option<String>,
    #[serde(default)]
    impersonate: bool,
    label: String,
}

/// Which tool a binding applies to.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GatewayClientDto {
    Codex,
    ClaudeCode,
}

impl From<GatewayClientDto> for GatewayClient {
    fn from(dto: GatewayClientDto) -> Self {
        match dto {
            GatewayClientDto::Codex => GatewayClient::Codex,
            GatewayClientDto::ClaudeCode => GatewayClient::ClaudeCode,
        }
    }
}

impl From<GatewayClient> for GatewayClientDto {
    fn from(client: GatewayClient) -> Self {
        match client {
            GatewayClient::Codex => GatewayClientDto::Codex,
            GatewayClient::ClaudeCode => GatewayClientDto::ClaudeCode,
        }
    }
}

/// Protocol the relay upstream speaks, chosen by the user when binding.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpstreamKindDto {
    /// OpenAI Chat Completions — needs translation for both tools.
    OpenaiChat,
    /// OpenAI Responses — native for Codex, passthrough.
    OpenaiResponses,
    /// Anthropic Messages — native for Claude Code, passthrough.
    Anthropic,
}

impl From<UpstreamKindDto> for UpstreamKind {
    fn from(dto: UpstreamKindDto) -> Self {
        match dto {
            UpstreamKindDto::OpenaiChat => UpstreamKind::OpenAiChat,
            UpstreamKindDto::OpenaiResponses => UpstreamKind::OpenAiResponses,
            UpstreamKindDto::Anthropic => UpstreamKind::Anthropic,
        }
    }
}

/// Where a binding's upstream key comes from: any relay-station account token
/// or an independent API key.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GatewaySourceDto {
    AccountToken {
        account_id: String,
        provider_id: String,
        token_id: i64,
        /// Relay node URL chosen in the UI; defaults to the provider domain.
        base_url: Option<String>,
    },
    IndependentKey {
        key_id: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct GatewayBindingDto {
    pub client: GatewayClientDto,
    /// Human-readable description of the bound key (name, never the key).
    pub label: String,
    pub model: Option<String>,
    pub upstream_kind: UpstreamKindDto,
    pub impersonate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct GatewayStatusDto {
    pub running: bool,
    /// `http://127.0.0.1:<port>` when running.
    pub base_url: Option<String>,
    pub bindings: Vec<GatewayBindingDto>,
}

/// Resolve a binding source into the upstream the gateway forwards to. This is
/// the single exhaustive place key shapes are read; adding a source variant
/// fails to compile until handled.
async fn resolve_upstream(
    source: &GatewaySourceDto,
    kind: UpstreamKind,
    model: Option<&str>,
    impersonate: bool,
    services: &Services,
    repositories: &Repositories,
) -> Result<(GatewayUpstream, String), CommandError> {
    match source {
        GatewaySourceDto::AccountToken {
            account_id,
            provider_id,
            token_id,
            base_url,
        } => {
            let account_id = AccountId::from_string(account_id);
            let token_id = neuradock_domain::token::TokenId::new(*token_id);
            let tokens = services
                .token
                .get_cached_tokens(&account_id)
                .await
                .map_err(CommandError::from)?;
            let token = tokens
                .iter()
                .find(|t| t.id() == &token_id)
                .ok_or_else(|| CommandError::not_found("Token not found"))?;

            let provider_id = ProviderId::from_string(provider_id);
            let provider = repositories
                .provider
                .find_by_id(&provider_id)
                .await
                .map_err(CommandError::from)?
                .ok_or_else(|| CommandError::not_found("Provider not found"))?;

            let base_url = base_url
                .clone()
                .unwrap_or_else(|| provider.domain().to_string());
            let label = format!("{} · {}", provider.name(), token.name());

            Ok((
                GatewayUpstream {
                    base_url,
                    api_key: token.key().to_string(),
                    kind,
                    model: model.map(str::to_string),
                    impersonate,
                },
                label,
            ))
        }
        GatewaySourceDto::IndependentKey { key_id } => {
            let id = IndependentKeyId::new(*key_id);
            let key = repositories
                .independent_key
                .find_by_id(&id)
                .await
                .map_err(CommandError::from)?
                .ok_or_else(|| {
                    CommandError::not_found(format!("Key with ID {} not found", key_id))
                })?;
            if !key.is_active() {
                return Err(CommandError::validation(
                    "Cannot bind an inactive API key. Please enable it first.",
                ));
            }
            Ok((
                GatewayUpstream {
                    base_url: key.base_url().to_string(),
                    api_key: key.api_key().to_string(),
                    kind,
                    model: model.map(str::to_string),
                    impersonate,
                },
                key.name().to_string(),
            ))
        }
    }
}

async fn status_of(state: &GatewayState) -> GatewayStatusDto {
    let server = state.server.lock().await;
    let meta = state.meta.lock().await;
    let mut bindings: Vec<GatewayBindingDto> = meta
        .iter()
        .map(|(client, meta)| GatewayBindingDto {
            client: (*client).into(),
            label: meta.label.clone(),
            model: meta.model.clone(),
            upstream_kind: meta.kind,
            impersonate: meta.impersonate,
        })
        .collect();
    bindings.sort_by_key(|b| b.client == GatewayClientDto::ClaudeCode);

    GatewayStatusDto {
        running: server.is_some(),
        base_url: server.as_ref().map(|handle| handle.base_url()),
        bindings,
    }
}

fn takeover_for(app: &tauri::AppHandle) -> Result<GatewayTakeover, CommandError> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| {
        CommandError::infrastructure(format!("Failed to get app data directory: {}", e))
    })?;
    Ok(GatewayTakeover::new(&app_data_dir))
}

/// Bind a client (Codex / Claude Code) to an upstream relay key. If the
/// gateway is running, the client's tool config is (re)taken over immediately.
#[tauri::command]
#[specta::specta]
pub async fn set_gateway_binding(
    client: GatewayClientDto,
    source: GatewaySourceDto,
    upstream_kind: UpstreamKindDto,
    model: Option<String>,
    impersonate: Option<bool>,
    app: tauri::AppHandle,
    state: State<'_, GatewayState>,
    services: State<'_, Services>,
    repositories: State<'_, Repositories>,
) -> Result<GatewayStatusDto, CommandError> {
    // Impersonation defaults on: the point of the gateway is to reach relays
    // that only admit recognised clients.
    let impersonate = impersonate.unwrap_or(true);
    let (upstream, label) = resolve_upstream(
        &source,
        upstream_kind.into(),
        model.as_deref(),
        impersonate,
        &services,
        &repositories,
    )
    .await?;
    let gateway_client: GatewayClient = client.into();

    state
        .bindings
        .write()
        .await
        .insert(gateway_client, upstream);
    state.meta.lock().await.insert(
        gateway_client,
        BindingMeta {
            label,
            source,
            kind: upstream_kind,
            model: model.clone(),
            impersonate,
        },
    );

    persist_bindings(&app, &state).await;

    // Refresh the takeover when the server is already running.
    let server = state.server.lock().await;
    if let Some(handle) = server.as_ref() {
        let takeover = takeover_for(&app)?;
        takeover
            .take_over(gateway_client, &handle.base_url())
            .map_err(|e| CommandError::infrastructure(e.to_string()))?;
    }
    drop(server);

    Ok(status_of(&state).await)
}

/// Remove a client's binding; restores its tool config if taken over.
#[tauri::command]
#[specta::specta]
pub async fn clear_gateway_binding(
    client: GatewayClientDto,
    app: tauri::AppHandle,
    state: State<'_, GatewayState>,
) -> Result<GatewayStatusDto, CommandError> {
    let gateway_client: GatewayClient = client.into();

    state.bindings.write().await.remove(&gateway_client);
    state.meta.lock().await.remove(&gateway_client);

    persist_bindings(&app, &state).await;

    let takeover = takeover_for(&app)?;
    takeover
        .restore(gateway_client)
        .map_err(|e| CommandError::infrastructure(e.to_string()))?;

    Ok(status_of(&state).await)
}

const BINDINGS_FILENAME: &str = "gateway_bindings.json";

/// Persist the current bindings (references only, never keys) to app data so
/// they survive a restart.
async fn persist_bindings(app: &tauri::AppHandle, state: &GatewayState) {
    let Ok(dir) = app.path().app_data_dir() else {
        return;
    };
    let meta = state.meta.lock().await;
    let bindings: Vec<PersistedBinding> = meta
        .iter()
        .map(|(client, meta)| PersistedBinding {
            client: (*client).into(),
            source: meta.source.clone(),
            kind: meta.kind,
            model: meta.model.clone(),
            impersonate: meta.impersonate,
            label: meta.label.clone(),
        })
        .collect();
    drop(meta);

    let path = dir.join(BINDINGS_FILENAME);
    match serde_json::to_string_pretty(&bindings) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                log::warn!("Failed to persist gateway bindings: {}", e);
            }
        }
        Err(e) => log::warn!("Failed to serialize gateway bindings: {}", e),
    }
}

/// Rebuild the binding table from disk on startup. Re-resolves each saved
/// reference into a live upstream (the key is fetched fresh); bindings whose
/// key was deleted or disabled are skipped with a log line. The gateway server
/// is NOT started — the user starts it explicitly.
pub async fn restore_gateway_bindings(
    app: &tauri::AppHandle,
    state: &GatewayState,
    services: &Services,
    repositories: &Repositories,
) {
    let Ok(dir) = app.path().app_data_dir() else {
        return;
    };
    let path = dir.join(BINDINGS_FILENAME);
    let Ok(content) = std::fs::read_to_string(&path) else {
        return;
    };
    let saved: Vec<PersistedBinding> = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("Failed to parse saved gateway bindings: {}", e);
            return;
        }
    };

    for binding in saved {
        match resolve_upstream(
            &binding.source,
            binding.kind.into(),
            binding.model.as_deref(),
            binding.impersonate,
            services,
            repositories,
        )
        .await
        {
            Ok((upstream, label)) => {
                let client: GatewayClient = binding.client.into();
                state.bindings.write().await.insert(client, upstream);
                state.meta.lock().await.insert(
                    client,
                    BindingMeta {
                        label,
                        source: binding.source,
                        kind: binding.kind,
                        model: binding.model,
                        impersonate: binding.impersonate,
                    },
                );
            }
            Err(e) => log::warn!(
                "Skipping saved gateway binding for {:?}: {}",
                binding.client,
                e
            ),
        }
    }
    log::info!("Restored gateway bindings from disk (server left stopped)");
}

/// Start the gateway server and take over every bound client's tool config.
#[tauri::command]
#[specta::specta]
pub async fn start_gateway_server(
    app: tauri::AppHandle,
    state: State<'_, GatewayState>,
) -> Result<GatewayStatusDto, CommandError> {
    let bound_clients: Vec<GatewayClient> = state.bindings.read().await.keys().copied().collect();
    if bound_clients.is_empty() {
        return Err(CommandError::validation(
            "No client is bound to a relay key yet. Bind one from an account token or API key first.",
        ));
    }

    let mut server = state.server.lock().await;
    if server.is_none() {
        let handle = start_gateway(state.bindings.clone(), 0)
            .await
            .map_err(|e| CommandError::infrastructure(format!("Failed to start gateway: {}", e)))?;
        *server = Some(handle);
    }
    let base_url = server.as_ref().expect("just started").base_url();
    drop(server);

    let takeover = takeover_for(&app)?;
    for client in bound_clients {
        takeover
            .take_over(client, &base_url)
            .map_err(|e| CommandError::infrastructure(e.to_string()))?;
    }

    Ok(status_of(&state).await)
}

/// Stop the gateway server and restore every taken-over tool config.
#[tauri::command]
#[specta::specta]
pub async fn stop_gateway_server(
    app: tauri::AppHandle,
    state: State<'_, GatewayState>,
) -> Result<GatewayStatusDto, CommandError> {
    let takeover = takeover_for(&app)?;
    takeover
        .restore_all()
        .map_err(|e| CommandError::infrastructure(e.to_string()))?;

    if let Some(handle) = state.server.lock().await.take() {
        handle.shutdown().await;
    }

    Ok(status_of(&state).await)
}

/// Report gateway state for the sidebar switch and panel.
#[tauri::command]
#[specta::specta]
pub async fn get_gateway_status(
    state: State<'_, GatewayState>,
) -> Result<GatewayStatusDto, CommandError> {
    Ok(status_of(&state).await)
}
