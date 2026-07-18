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

use crate::application::services::{GatewayTakeover, GATEWAY_PLACEHOLDER_KEY};
use crate::presentation::commands::agent_access::{
    AccessMethodDto, AgentAccessDto, AgentAccessSourceDto, AgentAccessState, AgentIdDto,
};
use crate::presentation::commands::token::ensure_account_token_usable;

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
    context1m: bool,
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
    context1m: bool,
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

impl From<GatewayClientDto> for AgentIdDto {
    fn from(dto: GatewayClientDto) -> Self {
        match dto {
            GatewayClientDto::Codex => AgentIdDto::Codex,
            GatewayClientDto::ClaudeCode => AgentIdDto::ClaudeCode,
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

fn upstream_kind_str(kind: UpstreamKindDto) -> String {
    match kind {
        UpstreamKindDto::OpenaiChat => "openai_chat",
        UpstreamKindDto::OpenaiResponses => "openai_responses",
        UpstreamKindDto::Anthropic => "anthropic",
    }
    .to_string()
}

fn effective_gateway_model(
    _client: GatewayClientDto,
    model: Option<&str>,
    _context1m: bool,
) -> Option<String> {
    // `[1m]` is a Claude Code configuration marker, not an upstream model id.
    // The takeover layer adds it to Claude's local settings so Claude Code can
    // emit the corresponding context beta header. The gateway binding itself
    // must keep the real upstream model name, separate from local capability
    // markers written into the tool configuration.
    model.map(str::to_string)
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

fn access_source_of(source: &GatewaySourceDto) -> AgentAccessSourceDto {
    match source {
        GatewaySourceDto::AccountToken {
            account_id,
            provider_id,
            token_id,
            base_url,
        } => AgentAccessSourceDto::AccountToken {
            account_id: account_id.clone(),
            provider_id: provider_id.clone(),
            token_id: *token_id,
            base_url: base_url.clone(),
        },
        GatewaySourceDto::IndependentKey { key_id } => {
            AgentAccessSourceDto::IndependentKey { key_id: *key_id }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct GatewayBindingDto {
    pub client: GatewayClientDto,
    /// Human-readable description of the bound key (name, never the key).
    pub label: String,
    pub model: Option<String>,
    pub context1m: bool,
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

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfigPreviewRequestDto {
    pub client: GatewayClientDto,
    pub method: AccessMethodDto,
    pub source: GatewaySourceDto,
    pub model: Option<String>,
    #[serde(default, rename = "context1m")]
    pub context_1m: bool,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AgentConfigPreviewFileDto {
    pub path: String,
    pub format: String,
    pub current_content: String,
    pub next_content: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AgentConfigPreviewDto {
    pub files: Vec<AgentConfigPreviewFileDto>,
    pub takeover_active: bool,
    pub gateway_running: bool,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GatewayBindingConfigDto {
    pub upstream_kind: UpstreamKindDto,
    pub model: Option<String>,
    #[serde(default, rename = "context1m")]
    pub context_1m: bool,
    pub impersonate: Option<bool>,
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
            let account = repositories
                .account
                .find_by_id(&account_id)
                .await
                .map_err(CommandError::from)?
                .ok_or_else(|| CommandError::not_found("Account not found"))?;
            let tokens = services
                .token
                .get_cached_tokens(&account_id)
                .await
                .map_err(CommandError::from)?;
            let token = tokens
                .iter()
                .find(|t| t.id() == &token_id)
                .ok_or_else(|| CommandError::not_found("Token not found"))?;
            ensure_account_token_usable(&account, token)?;

            let provider_id = ProviderId::from_string(provider_id);
            if account.provider_id() != &provider_id {
                return Err(CommandError::validation(
                    "The selected provider does not belong to this account.",
                ));
            }
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
            context1m: meta.context1m,
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

/// Build a read-only, redacted view of the files managed for an Agent. The
/// same config builders used by the write commands produce `next_content`, so
/// preview and save cannot drift apart.
#[tauri::command]
#[specta::specta]
pub async fn get_agent_config_preview(
    request: AgentConfigPreviewRequestDto,
    app: tauri::AppHandle,
    state: State<'_, GatewayState>,
    services: State<'_, Services>,
    repositories: State<'_, Repositories>,
) -> Result<AgentConfigPreviewDto, CommandError> {
    if request.context_1m && request.model.is_none() {
        return Err(CommandError::validation(
            "1M context requires selecting a model.",
        ));
    }

    let gateway_client: GatewayClient = request.client.into();
    let takeover = takeover_for(&app)?;
    let takeover_active = takeover.is_active_for(gateway_client);
    let gateway_base_url = state
        .server
        .lock()
        .await
        .as_ref()
        .map(|handle| handle.base_url());
    let gateway_running = gateway_base_url.is_some();

    let raw_files = match request.method {
        AccessMethodDto::Gateway => {
            let preview_base_url = gateway_base_url
                .as_deref()
                .unwrap_or("http://127.0.0.1:<port>");
            preview_with_key(
                request.client,
                GATEWAY_PLACEHOLDER_KEY,
                preview_base_url,
                request.model.as_deref(),
                request.context_1m,
                &services,
            )?
        }
        AccessMethodDto::Direct => {
            preview_direct(
                request.client,
                &request.source,
                request.model.as_deref(),
                request.context_1m,
                &services,
                &repositories,
            )
            .await?
        }
    };

    let files = raw_files
        .into_iter()
        .map(|(path, format, current, mut next)| {
            if request.method == AccessMethodDto::Gateway && !gateway_running {
                next = current.clone();
            }
            AgentConfigPreviewFileDto {
                path: abbreviate_home(&path),
                format: format.to_string(),
                current_content: redact_content(format, &current),
                next_content: redact_content(format, &next),
            }
        })
        .collect();

    Ok(AgentConfigPreviewDto {
        files,
        takeover_active,
        gateway_running,
    })
}

type RawPreviewFile = (std::path::PathBuf, &'static str, String, String);

fn preview_with_key(
    client: GatewayClientDto,
    api_key: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
    services: &Services,
) -> Result<Vec<RawPreviewFile>, CommandError> {
    match client {
        GatewayClientDto::Codex => services
            .codex_config
            .preview_global_with_key(api_key, base_url, model, context_1m)
            .map_err(CommandError::from),
        GatewayClientDto::ClaudeCode => services
            .claude_config
            .preview_global_with_key(api_key, base_url, model, context_1m)
            .map(|(path, current, next)| vec![(path, "json", current, next)])
            .map_err(CommandError::from),
    }
}

async fn preview_direct(
    client: GatewayClientDto,
    source: &GatewaySourceDto,
    model: Option<&str>,
    context_1m: bool,
    services: &Services,
    repositories: &Repositories,
) -> Result<Vec<RawPreviewFile>, CommandError> {
    match source {
        GatewaySourceDto::AccountToken {
            account_id,
            provider_id,
            token_id,
            base_url,
        } => {
            let account_id = AccountId::from_string(account_id);
            let token_id = neuradock_domain::token::TokenId::new(*token_id);
            let account = repositories
                .account
                .find_by_id(&account_id)
                .await
                .map_err(CommandError::from)?
                .ok_or_else(|| CommandError::not_found("Account not found"))?;
            let tokens = services
                .token
                .get_cached_tokens(&account_id)
                .await
                .map_err(CommandError::from)?;
            let token = tokens
                .iter()
                .find(|token| token.id() == &token_id)
                .ok_or_else(|| CommandError::not_found("Token not found"))?;
            ensure_account_token_usable(&account, token)?;

            let provider_id_obj = ProviderId::from_string(provider_id);
            if account.provider_id() != &provider_id_obj {
                return Err(CommandError::validation(
                    "The selected provider does not belong to this account.",
                ));
            }
            let provider = repositories
                .provider
                .find_by_id(&provider_id_obj)
                .await
                .map_err(CommandError::from)?
                .ok_or_else(|| CommandError::not_found("Provider not found"))?;
            let base_url = base_url.as_deref().unwrap_or_else(|| provider.domain());

            match client {
                GatewayClientDto::Codex => services
                    .codex_config
                    .preview_global(
                        token,
                        provider.id().as_str(),
                        provider.name(),
                        base_url,
                        model,
                        context_1m,
                    )
                    .map_err(CommandError::from),
                GatewayClientDto::ClaudeCode => services
                    .claude_config
                    .preview_global_with_key(token.key(), base_url, model, context_1m)
                    .map(|(path, current, next)| vec![(path, "json", current, next)])
                    .map_err(CommandError::from),
            }
        }
        GatewaySourceDto::IndependentKey { key_id } => {
            let key = repositories
                .independent_key
                .find_by_id(&IndependentKeyId::new(*key_id))
                .await
                .map_err(CommandError::from)?
                .ok_or_else(|| CommandError::not_found("Key not found"))?;
            if !key.is_active() {
                return Err(CommandError::validation(
                    "Cannot configure an inactive API key.",
                ));
            }
            preview_with_key(
                client,
                key.api_key(),
                key.base_url(),
                model,
                context_1m,
                services,
            )
        }
    }
}

fn abbreviate_home(path: &std::path::Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(relative) = path.strip_prefix(home) {
            return format!("~/{}", relative.display());
        }
    }
    path.display().to_string()
}

fn redact_content(format: &str, content: &str) -> String {
    if content.trim().is_empty() {
        return content.to_string();
    }
    if format == "toml" {
        return redact_toml_content(content);
    }
    if format != "json" {
        return content.to_string();
    }
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(content) else {
        return content.to_string();
    };
    redact_json_value(&mut value);
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| content.to_string())
}

fn redact_toml_content(content: &str) -> String {
    content
        .lines()
        .map(|line| {
            let Some((key, _value)) = line.split_once('=') else {
                return line.to_string();
            };
            let normalized = key.trim().trim_matches(['\'', '"']).to_ascii_uppercase();
            if normalized.contains("TOKEN")
                || normalized.contains("API_KEY")
                || normalized.contains("SECRET")
                || normalized.contains("PASSWORD")
            {
                format!("{}= \"••••••••\"", key)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn redact_json_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, value) in object {
                let normalized = key.to_ascii_uppercase();
                if normalized.contains("TOKEN")
                    || normalized.contains("API_KEY")
                    || normalized.contains("SECRET")
                    || normalized.contains("PASSWORD")
                {
                    if value.is_string() {
                        *value = serde_json::Value::String("••••••••".to_string());
                        continue;
                    }
                }
                redact_json_value(value);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                redact_json_value(value);
            }
        }
        _ => {}
    }
}

/// Bind a client (Codex / Claude Code) to an upstream relay key. If the
/// gateway is running, the client's tool config is (re)taken over immediately.
#[tauri::command]
#[specta::specta]
pub async fn set_gateway_binding(
    client: GatewayClientDto,
    source: GatewaySourceDto,
    config: GatewayBindingConfigDto,
    app: tauri::AppHandle,
    state: State<'_, GatewayState>,
    access: State<'_, AgentAccessState>,
    services: State<'_, Services>,
    repositories: State<'_, Repositories>,
) -> Result<GatewayStatusDto, CommandError> {
    // Impersonation defaults on: the point of the gateway is to reach relays
    // that only admit recognised clients.
    let GatewayBindingConfigDto {
        upstream_kind,
        model,
        context_1m: context1m,
        impersonate,
    } = config;
    let impersonate = impersonate.unwrap_or(true);
    if context1m && model.is_none() {
        return Err(CommandError::validation(
            "1M context requires selecting a model.",
        ));
    }
    let effective_model = effective_gateway_model(client, model.as_deref(), context1m);
    let (upstream, label) = resolve_upstream(
        &source,
        upstream_kind.into(),
        effective_model.as_deref(),
        impersonate,
        &services,
        &repositories,
    )
    .await?;
    let gateway_client: GatewayClient = client.into();
    let access_source = access_source_of(&source);

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
            context1m,
            impersonate,
        },
    );

    persist_bindings(&app, &state).await;

    // Mirror into the access record so the status card shows a consistent view.
    let label_for_record = state
        .meta
        .lock()
        .await
        .get(&gateway_client)
        .map(|m| m.label.clone())
        .unwrap_or_default();
    access.set(
        &app,
        AgentAccessDto {
            agent: client.into(),
            method: AccessMethodDto::Gateway,
            label: label_for_record,
            model: model.clone(),
            upstream_kind: Some(upstream_kind_str(upstream_kind)),
            impersonate: Some(impersonate),
            source: Some(access_source),
            context1m: Some(context1m),
        },
    );

    // Refresh the takeover when the server is already running.
    let server = state.server.lock().await;
    if let Some(handle) = server.as_ref() {
        let takeover = takeover_for(&app)?;
        takeover
            .take_over(
                gateway_client,
                &handle.base_url(),
                model.as_deref(),
                context1m,
            )
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
    access: State<'_, AgentAccessState>,
) -> Result<GatewayStatusDto, CommandError> {
    let gateway_client: GatewayClient = client.into();

    state.bindings.write().await.remove(&gateway_client);
    state.meta.lock().await.remove(&gateway_client);

    persist_bindings(&app, &state).await;
    access.remove(&app, client.into());

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
            context1m: meta.context1m,
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
    access: &AgentAccessState,
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
        let effective_model =
            effective_gateway_model(binding.client, binding.model.as_deref(), binding.context1m);
        match resolve_upstream(
            &binding.source,
            binding.kind.into(),
            effective_model.as_deref(),
            binding.impersonate,
            services,
            repositories,
        )
        .await
        {
            Ok((upstream, label)) => {
                let client: GatewayClient = binding.client.into();
                let access_source = access_source_of(&binding.source);
                state.bindings.write().await.insert(client, upstream);
                state.meta.lock().await.insert(
                    client,
                    BindingMeta {
                        label: label.clone(),
                        source: binding.source,
                        kind: binding.kind,
                        model: binding.model.clone(),
                        context1m: binding.context1m,
                        impersonate: binding.impersonate,
                    },
                );
                access.set(
                    app,
                    AgentAccessDto {
                        agent: binding.client.into(),
                        method: AccessMethodDto::Gateway,
                        label,
                        model: binding.model,
                        upstream_kind: Some(upstream_kind_str(binding.kind)),
                        impersonate: Some(binding.impersonate),
                        source: Some(access_source),
                        context1m: Some(binding.context1m),
                    },
                );
            }
            Err(e) => {
                access.remove(app, binding.client.into());
                log::warn!(
                    "Skipping saved gateway binding for {:?}: {}",
                    binding.client,
                    e
                );
            }
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
    let takeover_configs: Vec<(GatewayClient, Option<String>, bool)> = state
        .meta
        .lock()
        .await
        .iter()
        .map(|(client, meta)| (*client, meta.model.clone(), meta.context1m))
        .collect();
    if takeover_configs.is_empty() {
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
    for (client, model, context1m) in takeover_configs {
        takeover
            .take_over(client, &base_url, model.as_deref(), context1m)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_upstream_model_never_carries_1m_suffix() {
        assert_eq!(
            effective_gateway_model(GatewayClientDto::ClaudeCode, Some("claude-sonnet-4"), true),
            Some("claude-sonnet-4".to_string())
        );
        assert_eq!(
            effective_gateway_model(GatewayClientDto::Codex, Some("gpt-5"), true),
            Some("gpt-5".to_string())
        );
    }

    #[test]
    fn old_gateway_bindings_default_to_standard_context() {
        let json = r#"{
            "client": "codex",
            "source": { "type": "independent_key", "key_id": 1 },
            "kind": "openai_chat",
            "model": null,
            "impersonate": true,
            "label": "Key"
        }"#;
        let binding: PersistedBinding = serde_json::from_str(json).unwrap();
        assert!(!binding.context1m);
    }

    #[test]
    fn config_preview_redacts_sensitive_json_values() {
        let content = r#"{
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "secret",
                "SAFE_VALUE": "visible"
            },
            "OPENAI_API_KEY": "secret-two"
        }"#;
        let redacted = redact_content("json", content);

        assert!(!redacted.contains("secret"));
        assert!(redacted.contains("visible"));
        assert!(redacted.contains("••••••••"));
    }

    #[test]
    fn config_preview_redacts_sensitive_toml_values() {
        let content = "model = \"gpt-5\"\napi_key = \"secret\"\nenv_key = \"OPENAI_API_KEY\"\n";
        let redacted = redact_content("toml", content);

        assert!(!redacted.contains("\"secret\""));
        assert!(redacted.contains("model = \"gpt-5\""));
        assert!(redacted.contains("env_key = \"OPENAI_API_KEY\""));
    }
}
