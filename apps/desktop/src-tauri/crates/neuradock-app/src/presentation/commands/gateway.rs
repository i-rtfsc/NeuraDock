use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Manager, State};
use tokio::sync::{Mutex, RwLock};

use crate::presentation::error::CommandError;
use crate::presentation::state::{Repositories, Services};
use neuradock_domain::independent_key::IndependentKeyId;
use neuradock_domain::shared::{AccountId, ProviderId};
use neuradock_infrastructure::fs_safe::atomic_write;
use neuradock_infrastructure::gateway::{
    start_gateway_with_runtime, CircuitState, GatewayBindings, GatewayClient, GatewayHandle,
    GatewayRuntime, GatewayUpstream, UpstreamKind,
};

use crate::application::services::{ClaudeConfigService, CodexConfigService, GatewayTakeover};
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
    runtime: Arc<GatewayRuntime>,
    codex_operation: Mutex<()>,
    claude_operation: Mutex<()>,
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
    fn operation_lock(&self, client: GatewayClient) -> &Mutex<()> {
        match client {
            GatewayClient::Codex => &self.codex_operation,
            GatewayClient::ClaudeCode => &self.claude_operation,
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
    pub upstream: Option<GatewayUpstreamStatusDto>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum CircuitStateDto {
    Closed,
    Open,
    HalfOpen,
}

impl From<CircuitState> for CircuitStateDto {
    fn from(value: CircuitState) -> Self {
        match value {
            CircuitState::Closed => Self::Closed,
            CircuitState::Open => Self::Open,
            CircuitState::HalfOpen => Self::HalfOpen,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct GatewayUpstreamStatusDto {
    pub id: String,
    pub label: String,
    pub circuit_state: CircuitStateDto,
    pub consecutive_failures: u32,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub average_latency_ms: u64,
    pub average_first_event_latency_ms: u64,
    pub last_error: Option<String>,
    pub last_error_at_ms: Option<u64>,
    pub recovery_at_ms: Option<u64>,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentConfigurationKindDto {
    Managed,
    Manual,
    Changed,
    Unconfigured,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct AgentConfigurationStateDto {
    pub agent: GatewayClientDto,
    pub state: AgentConfigurationKindDto,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub context1m: bool,
    pub credential_present: bool,
    pub config_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct AgentConfigFingerprint {
    managed_values: BTreeMap<String, String>,
    endpoint: Option<String>,
    model: Option<String>,
    context1m: bool,
    credential_present: bool,
    configured: bool,
    parse_ok: bool,
    paths: Vec<PathBuf>,
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
            let upstream_id = format!("account:{account_id}:{token_id}");
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
            let label = format!(
                "{} · {} · {}",
                provider.name(),
                account.name(),
                token.name()
            );

            Ok((
                GatewayUpstream {
                    id: upstream_id,
                    label: label.clone(),
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
                    id: format!("independent:{key_id}"),
                    label: key.name().to_string(),
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
    let (running, base_url) = {
        let server = state.server.lock().await;
        (
            server.is_some(),
            server.as_ref().map(|handle| handle.base_url()),
        )
    };
    let meta = state.meta.lock().await.clone();
    let upstreams = state.bindings.read().await.clone();
    let mut bindings = Vec::new();
    for (client, meta) in meta {
        let upstream = if let Some(configured) = upstreams.get(&client) {
            let snapshot = state.runtime.snapshot(client, configured).await;
            Some(GatewayUpstreamStatusDto {
                id: snapshot.id,
                label: snapshot.label,
                circuit_state: snapshot.state.into(),
                consecutive_failures: snapshot.consecutive_failures,
                total_requests: snapshot.total_requests,
                successful_requests: snapshot.successful_requests,
                failed_requests: snapshot.failed_requests,
                input_tokens: snapshot.usage.input_tokens,
                output_tokens: snapshot.usage.output_tokens,
                cache_read_tokens: snapshot.usage.cache_read_tokens,
                cache_creation_tokens: snapshot.usage.cache_creation_tokens,
                average_latency_ms: snapshot.average_latency_ms,
                average_first_event_latency_ms: snapshot.average_first_event_latency_ms,
                last_error: snapshot.last_error,
                last_error_at_ms: snapshot.last_error_at_ms,
                recovery_at_ms: snapshot.recovery_at_ms,
            })
        } else {
            None
        };
        bindings.push(GatewayBindingDto {
            client: client.into(),
            label: meta.label,
            model: meta.model,
            context1m: meta.context1m,
            upstream_kind: meta.kind,
            impersonate: meta.impersonate,
            upstream,
        });
    }
    bindings.sort_by_key(|b| b.client == GatewayClientDto::ClaudeCode);

    GatewayStatusDto {
        running,
        base_url,
        bindings,
    }
}

fn takeover_for(app: &tauri::AppHandle) -> Result<GatewayTakeover, CommandError> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| {
        CommandError::infrastructure(format!("Failed to get app data directory: {}", e))
    })?;
    Ok(GatewayTakeover::new(&app_data_dir))
}

fn client_for_agent(agent: AgentIdDto) -> GatewayClientDto {
    match agent {
        AgentIdDto::Codex => GatewayClientDto::Codex,
        AgentIdDto::ClaudeCode => GatewayClientDto::ClaudeCode,
    }
}

fn source_from_access(source: &AgentAccessSourceDto) -> GatewaySourceDto {
    match source {
        AgentAccessSourceDto::AccountToken {
            account_id,
            provider_id,
            token_id,
            base_url,
        } => GatewaySourceDto::AccountToken {
            account_id: account_id.clone(),
            provider_id: provider_id.clone(),
            token_id: *token_id,
            base_url: base_url.clone(),
        },
        AgentAccessSourceDto::IndependentKey { key_id } => {
            GatewaySourceDto::IndependentKey { key_id: *key_id }
        }
    }
}

fn canonical_url(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches('/');
    let Ok(mut parsed) = url::Url::parse(trimmed) else {
        return trimmed.to_string();
    };
    parsed.set_query(None);
    parsed.set_fragment(None);
    let _ = parsed.set_username("");
    let _ = parsed.set_password(None);
    parsed.to_string().trim_end_matches('/').to_string()
}

fn display_endpoint(value: &str) -> String {
    let Ok(parsed) = url::Url::parse(value) else {
        return value.to_string();
    };
    match (parsed.host_str(), parsed.port()) {
        (Some(host), Some(port)) => format!("{host}:{port}"),
        (Some(host), None) => host.to_string(),
        _ => value.to_string(),
    }
}

fn model_without_context_marker(value: &str) -> (String, bool) {
    let trimmed = value.trim();
    if trimmed.len() >= 4 && trimmed[trimmed.len() - 4..].eq_ignore_ascii_case("[1m]") {
        (trimmed[..trimmed.len() - 4].trim_end().to_string(), true)
    } else {
        (trimmed.to_string(), false)
    }
}

fn claude_fingerprint(content: &str, path: PathBuf) -> anyhow::Result<AgentConfigFingerprint> {
    let mut result = AgentConfigFingerprint {
        parse_ok: true,
        paths: vec![path],
        ..Default::default()
    };
    if content.trim().is_empty() {
        return Ok(result);
    }

    let root: serde_json::Value = serde_json::from_str(content)?;
    let Some(env) = root.get("env").and_then(serde_json::Value::as_object) else {
        return Ok(result);
    };
    const KEYS: &[&str] = &[
        "ANTHROPIC_AUTH_TOKEN",
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_BASE_URL",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        "ANTHROPIC_DEFAULT_OPUS_MODEL",
    ];
    for key in KEYS {
        if let Some(value) = env
            .get(*key)
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            let normalized = if *key == "ANTHROPIC_BASE_URL" {
                canonical_url(value)
            } else {
                value.trim().to_string()
            };
            result.managed_values.insert((*key).to_string(), normalized);
        }
    }

    result.credential_present = result.managed_values.contains_key("ANTHROPIC_AUTH_TOKEN")
        || result.managed_values.contains_key("ANTHROPIC_API_KEY");
    if let Some(base_url) = result.managed_values.get("ANTHROPIC_BASE_URL") {
        result.endpoint = Some(display_endpoint(base_url));
    }
    let model = [
        "ANTHROPIC_DEFAULT_OPUS_MODEL",
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    ]
    .into_iter()
    .find_map(|key| result.managed_values.get(key));
    if let Some(model) = model {
        let (model, context1m) = model_without_context_marker(model);
        result.model = Some(model);
        result.context1m = context1m;
    }
    result.configured =
        result.credential_present || result.endpoint.is_some() || result.model.is_some();
    Ok(result)
}

fn codex_fingerprint(
    config_content: &str,
    auth_content: &str,
    config_path: PathBuf,
    auth_path: PathBuf,
) -> anyhow::Result<AgentConfigFingerprint> {
    let mut result = AgentConfigFingerprint {
        parse_ok: true,
        paths: vec![config_path, auth_path],
        ..Default::default()
    };
    let doc = if config_content.trim().is_empty() {
        toml_edit::DocumentMut::new()
    } else {
        config_content.parse::<toml_edit::DocumentMut>()?
    };
    for key in [
        "model",
        "model_provider",
        "preferred_auth_method",
        "model_context_window",
    ] {
        if let Some(item) = doc.get(key) {
            if let Some(value) = item.as_str() {
                result
                    .managed_values
                    .insert(format!("config.{key}"), value.to_string());
            } else if let Some(value) = item.as_integer() {
                result
                    .managed_values
                    .insert(format!("config.{key}"), value.to_string());
            }
        }
    }

    if let Some(provider_slug) = doc.get("model_provider").and_then(toml_edit::Item::as_str) {
        if let Some(provider) = doc
            .get("model_providers")
            .and_then(toml_edit::Item::as_table)
            .and_then(|providers| providers.get(provider_slug))
            .and_then(toml_edit::Item::as_table)
        {
            for key in ["name", "base_url", "wire_api"] {
                if let Some(value) = provider.get(key).and_then(toml_edit::Item::as_str) {
                    let normalized = if key == "base_url" {
                        canonical_url(value)
                    } else {
                        value.to_string()
                    };
                    result
                        .managed_values
                        .insert(format!("provider.{key}"), normalized);
                }
            }
        }
    }

    let auth = if auth_content.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str::<serde_json::Value>(auth_content)?
    };
    if let Some(api_key) = auth
        .get("OPENAI_API_KEY")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        result.managed_values.insert(
            "auth.OPENAI_API_KEY".to_string(),
            api_key.trim().to_string(),
        );
        result.credential_present = true;
    }
    if auth.get("tokens").is_some_and(|tokens| !tokens.is_null()) {
        result
            .managed_values
            .insert("auth.tokens_present".to_string(), "true".to_string());
        result.credential_present = true;
    }

    result.endpoint = result
        .managed_values
        .get("provider.base_url")
        .map(|value| display_endpoint(value));
    result.model = result.managed_values.get("config.model").cloned();
    result.context1m = result
        .managed_values
        .get("config.model_context_window")
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|value| value >= 1_000_000);
    result.configured = result.credential_present
        || result.endpoint.is_some()
        || result.model.is_some()
        || result.managed_values.contains_key("config.model_provider");
    Ok(result)
}

fn read_agent_fingerprint(
    client: GatewayClientDto,
) -> Result<AgentConfigFingerprint, CommandError> {
    match client {
        GatewayClientDto::ClaudeCode => {
            let path = ClaudeConfigService::managed_path().map_err(CommandError::from)?;
            let content = if path.exists() {
                fs::read_to_string(&path).map_err(|error| {
                    CommandError::infrastructure(format!(
                        "Failed to read {}: {error}",
                        path.display()
                    ))
                })?
            } else {
                String::new()
            };
            let fallback_path = path.clone();
            Ok(
                claude_fingerprint(&content, path).unwrap_or_else(|_| AgentConfigFingerprint {
                    parse_ok: false,
                    paths: vec![fallback_path],
                    ..Default::default()
                }),
            )
        }
        GatewayClientDto::Codex => {
            let (config_path, auth_path) =
                CodexConfigService::managed_paths().map_err(CommandError::from)?;
            let config_content = if config_path.exists() {
                fs::read_to_string(&config_path).map_err(|error| {
                    CommandError::infrastructure(format!(
                        "Failed to read {}: {error}",
                        config_path.display()
                    ))
                })?
            } else {
                String::new()
            };
            let auth_content = if auth_path.exists() {
                fs::read_to_string(&auth_path).map_err(|error| {
                    CommandError::infrastructure(format!(
                        "Failed to read {}: {error}",
                        auth_path.display()
                    ))
                })?
            } else {
                String::new()
            };
            let fallback_paths = vec![config_path.clone(), auth_path.clone()];
            Ok(
                codex_fingerprint(&config_content, &auth_content, config_path, auth_path)
                    .unwrap_or_else(|_| AgentConfigFingerprint {
                        parse_ok: false,
                        paths: fallback_paths,
                        ..Default::default()
                    }),
            )
        }
    }
}

fn fingerprint_from_preview(
    client: GatewayClientDto,
    files: &[RawPreviewFile],
) -> Result<AgentConfigFingerprint, CommandError> {
    match client {
        GatewayClientDto::ClaudeCode => {
            let (path, _, _, next) = files
                .first()
                .ok_or_else(|| CommandError::infrastructure("Claude preview is empty"))?;
            claude_fingerprint(next, path.clone()).map_err(CommandError::from)
        }
        GatewayClientDto::Codex => {
            let config = files
                .iter()
                .find(|(_, format, _, _)| *format == "toml")
                .ok_or_else(|| CommandError::infrastructure("Codex config preview is missing"))?;
            let auth = files
                .iter()
                .find(|(_, format, _, _)| *format == "json")
                .ok_or_else(|| CommandError::infrastructure("Codex auth preview is missing"))?;
            codex_fingerprint(&config.3, &auth.3, config.0.clone(), auth.0.clone())
                .map_err(CommandError::from)
        }
    }
}

async fn expected_fingerprint(
    record: &AgentAccessDto,
    gateway_base_url: Option<&str>,
    gateway_credential: Option<&str>,
    services: &Services,
    repositories: &Repositories,
) -> Result<Option<AgentConfigFingerprint>, CommandError> {
    let client = client_for_agent(record.agent);
    let context1m = record.context1m.unwrap_or(false);
    let files = match record.method {
        AccessMethodDto::Gateway => {
            let (Some(base_url), Some(credential)) = (gateway_base_url, gateway_credential) else {
                return Ok(None);
            };
            preview_with_key(
                client,
                credential,
                base_url,
                record.model.as_deref(),
                context1m,
                services,
            )?
        }
        AccessMethodDto::Direct => {
            let source = record
                .source
                .as_ref()
                .ok_or_else(|| CommandError::validation("Access source is missing"))?;
            preview_direct(
                client,
                &source_from_access(source),
                record.model.as_deref(),
                context1m,
                services,
                repositories,
            )
            .await?
        }
    };
    fingerprint_from_preview(client, &files).map(Some)
}

#[tauri::command]
#[specta::specta]
pub async fn get_agent_configuration_states(
    state: State<'_, GatewayState>,
    access: State<'_, AgentAccessState>,
    services: State<'_, Services>,
    repositories: State<'_, Repositories>,
) -> Result<Vec<AgentConfigurationStateDto>, CommandError> {
    let gateway = state
        .server
        .lock()
        .await
        .as_ref()
        .map(|handle| (handle.base_url(), handle.local_credential().to_string()));
    let mut states = Vec::new();

    for agent in [AgentIdDto::Codex, AgentIdDto::ClaudeCode] {
        let client = client_for_agent(agent);
        let actual = read_agent_fingerprint(client)?;
        let record = access.get(agent);
        let state_kind = if let Some(record) = record.as_ref() {
            match expected_fingerprint(
                record,
                gateway.as_ref().map(|(base_url, _)| base_url.as_str()),
                gateway.as_ref().map(|(_, credential)| credential.as_str()),
                &services,
                &repositories,
            )
            .await
            {
                Ok(None) => AgentConfigurationKindDto::Managed,
                Ok(Some(expected))
                    if actual.parse_ok && actual.managed_values == expected.managed_values =>
                {
                    AgentConfigurationKindDto::Managed
                }
                _ => AgentConfigurationKindDto::Changed,
            }
        } else if actual.parse_ok && actual.configured {
            AgentConfigurationKindDto::Manual
        } else {
            AgentConfigurationKindDto::Unconfigured
        };

        states.push(AgentConfigurationStateDto {
            agent: client,
            state: state_kind,
            endpoint: actual.endpoint,
            model: actual.model,
            context1m: actual.context1m,
            credential_present: actual.credential_present,
            config_paths: actual
                .paths
                .iter()
                .map(|path| abbreviate_home(path))
                .collect(),
        });
    }
    Ok(states)
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
    let gateway = state
        .server
        .lock()
        .await
        .as_ref()
        .map(|handle| (handle.base_url(), handle.local_credential().to_string()));
    let gateway_running = gateway.is_some();

    let raw_files = match request.method {
        AccessMethodDto::Gateway => {
            let preview_base_url = gateway
                .as_ref()
                .map(|(base_url, _)| base_url.as_str())
                .unwrap_or("http://127.0.0.1:<port>");
            let preview_credential = gateway
                .as_ref()
                .map(|(_, credential)| credential.as_str())
                .unwrap_or("ndg_preview_credential");
            preview_with_key(
                request.client,
                preview_credential,
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

async fn restore_binding_memory(
    state: &GatewayState,
    client: GatewayClient,
    upstream: Option<GatewayUpstream>,
    meta: Option<BindingMeta>,
) {
    let mut bindings = state.bindings.write().await;
    match upstream {
        Some(upstream) => {
            bindings.insert(client, upstream);
        }
        None => {
            bindings.remove(&client);
        }
    }
    drop(bindings);

    let mut metadata = state.meta.lock().await;
    match meta {
        Some(meta) => {
            metadata.insert(client, meta);
        }
        None => {
            metadata.remove(&client);
        }
    }
}

fn restore_access_record(
    app: &tauri::AppHandle,
    access: &AgentAccessState,
    agent: AgentIdDto,
    record: Option<AgentAccessDto>,
) -> Result<(), CommandError> {
    match record {
        Some(record) => access.set(app, record),
        None => access.remove(app, agent),
    }
}

async fn rollback_binding_change(
    app: &tauri::AppHandle,
    state: &GatewayState,
    access: &AgentAccessState,
    client: GatewayClient,
    agent: AgentIdDto,
    old_upstream: Option<GatewayUpstream>,
    old_meta: Option<BindingMeta>,
    old_access: Option<AgentAccessDto>,
) {
    restore_binding_memory(state, client, old_upstream, old_meta).await;
    if let Err(error) = persist_bindings(app, state).await {
        log::error!("Failed to roll back gateway bindings: {}", error);
    }
    if let Err(error) = restore_access_record(app, access, agent, old_access) {
        log::error!("Failed to roll back agent access record: {}", error);
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
    let agent: AgentIdDto = client.into();
    let _operation = state.operation_lock(gateway_client).lock().await;
    let access_source = access_source_of(&source);
    let old_upstream = state.bindings.read().await.get(&gateway_client).cloned();
    let old_meta = state.meta.lock().await.get(&gateway_client).cloned();
    let old_access = access.get(agent);
    let new_meta = BindingMeta {
        label: label.clone(),
        source,
        kind: upstream_kind,
        model: model.clone(),
        context1m,
        impersonate,
    };
    let new_access = AgentAccessDto {
        agent,
        method: AccessMethodDto::Gateway,
        label,
        model: model.clone(),
        upstream_kind: Some(upstream_kind_str(upstream_kind)),
        impersonate: Some(impersonate),
        source: Some(access_source),
        context1m: Some(context1m),
    };

    state
        .bindings
        .write()
        .await
        .insert(gateway_client, upstream);
    state.meta.lock().await.insert(gateway_client, new_meta);

    if let Err(error) = persist_bindings(&app, &state).await {
        restore_binding_memory(&state, gateway_client, old_upstream, old_meta).await;
        return Err(error);
    }

    // Mirror into the access record so the status card shows a consistent view.
    if let Err(error) = access.set(&app, new_access) {
        rollback_binding_change(
            &app,
            &state,
            &access,
            gateway_client,
            agent,
            old_upstream,
            old_meta,
            old_access,
        )
        .await;
        return Err(error);
    }

    // Refresh the takeover when the server is already running.
    let server = state.server.lock().await;
    if let Some(handle) = server.as_ref() {
        let takeover = match takeover_for(&app) {
            Ok(takeover) => takeover,
            Err(error) => {
                drop(server);
                rollback_binding_change(
                    &app,
                    &state,
                    &access,
                    gateway_client,
                    agent,
                    old_upstream,
                    old_meta,
                    old_access,
                )
                .await;
                return Err(error);
            }
        };
        if let Err(error) = takeover.take_over(
            gateway_client,
            &handle.base_url(),
            handle.local_credential(),
            model.as_deref(),
            context1m,
        ) {
            drop(server);
            rollback_binding_change(
                &app,
                &state,
                &access,
                gateway_client,
                agent,
                old_upstream,
                old_meta,
                old_access,
            )
            .await;
            return Err(CommandError::infrastructure(error.to_string()));
        }
    }
    drop(server);
    state.runtime.reset(gateway_client).await;

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
    let agent: AgentIdDto = client.into();
    let _operation = state.operation_lock(gateway_client).lock().await;
    let old_upstream = state.bindings.read().await.get(&gateway_client).cloned();
    let old_meta = state.meta.lock().await.get(&gateway_client).cloned();
    let old_access = access.get(agent);

    state.bindings.write().await.remove(&gateway_client);
    state.meta.lock().await.remove(&gateway_client);

    if let Err(error) = persist_bindings(&app, &state).await {
        restore_binding_memory(&state, gateway_client, old_upstream, old_meta).await;
        return Err(error);
    }
    if let Err(error) = access.remove(&app, agent) {
        rollback_binding_change(
            &app,
            &state,
            &access,
            gateway_client,
            agent,
            old_upstream,
            old_meta,
            old_access,
        )
        .await;
        return Err(error);
    }

    let takeover = match takeover_for(&app) {
        Ok(takeover) => takeover,
        Err(error) => {
            rollback_binding_change(
                &app,
                &state,
                &access,
                gateway_client,
                agent,
                old_upstream,
                old_meta,
                old_access,
            )
            .await;
            return Err(error);
        }
    };
    if let Err(error) = takeover.restore(gateway_client) {
        rollback_binding_change(
            &app,
            &state,
            &access,
            gateway_client,
            agent,
            old_upstream,
            old_meta,
            old_access,
        )
        .await;
        return Err(CommandError::infrastructure(error.to_string()));
    }
    state.runtime.reset(gateway_client).await;

    Ok(status_of(&state).await)
}

const BINDINGS_FILENAME: &str = "gateway_bindings.json";
const RUNTIME_FILENAME: &str = "gateway_runtime.json";

/// Persist the current bindings (references only, never keys) to app data so
/// they survive a restart.
async fn persist_bindings(
    app: &tauri::AppHandle,
    state: &GatewayState,
) -> Result<(), CommandError> {
    let dir = app.path().app_data_dir().map_err(|error| {
        CommandError::infrastructure(format!("Failed to get app data directory: {error}"))
    })?;
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
    let json = serde_json::to_string_pretty(&bindings)
        .map_err(|error| CommandError::infrastructure(error.to_string()))?;
    atomic_write(&path, &json, false)
        .map_err(|error| CommandError::infrastructure(error.to_string()))
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
    state
        .runtime
        .configure_persistence(dir.join(RUNTIME_FILENAME))
        .await;
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
                if let Err(error) = access.set(
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
                ) {
                    log::warn!(
                        "Failed to restore access record for {:?}: {}",
                        binding.client,
                        error
                    );
                }
            }
            Err(e) => {
                if let Err(remove_error) = access.remove(app, binding.client.into()) {
                    log::warn!(
                        "Failed to remove stale access record for {:?}: {}",
                        binding.client,
                        remove_error
                    );
                }
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
    let _codex_operation = state.codex_operation.lock().await;
    let _claude_operation = state.claude_operation.lock().await;
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

    if state.server.lock().await.is_some() {
        return Ok(status_of(&state).await);
    }
    let takeover = takeover_for(&app)?;
    let handle = start_gateway_with_runtime(state.bindings.clone(), state.runtime.clone(), 0)
        .await
        .map_err(|e| CommandError::infrastructure(format!("Failed to start gateway: {}", e)))?;
    let mut server = state.server.lock().await;
    *server = Some(handle);
    let handle = server.as_ref().expect("just started");
    let base_url = handle.base_url();
    let local_credential = handle.local_credential().to_string();
    drop(server);

    let mut taken_over = Vec::new();
    for (client, model, context1m) in takeover_configs {
        if let Err(error) = takeover.take_over(
            client,
            &base_url,
            &local_credential,
            model.as_deref(),
            context1m,
        ) {
            for restored_client in taken_over.into_iter().rev() {
                if let Err(restore_error) = takeover.restore(restored_client) {
                    log::error!(
                        "Failed to roll back gateway startup for {}: {}",
                        restored_client.display_name(),
                        restore_error
                    );
                }
            }
            if let Some(handle) = state.server.lock().await.take() {
                handle.shutdown().await;
            }
            return Err(CommandError::infrastructure(error.to_string()));
        }
        taken_over.push(client);
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
    let _codex_operation = state.codex_operation.lock().await;
    let _claude_operation = state.claude_operation.lock().await;
    let takeover = takeover_for(&app)?;
    let gateway = state
        .server
        .lock()
        .await
        .as_ref()
        .map(|handle| (handle.base_url(), handle.local_credential().to_string()));
    let configs: Vec<(GatewayClient, Option<String>, bool)> = state
        .meta
        .lock()
        .await
        .iter()
        .map(|(client, meta)| (*client, meta.model.clone(), meta.context1m))
        .collect();
    let mut restored = Vec::new();
    for (client, _, _) in &configs {
        if !takeover.is_active_for(*client) {
            continue;
        }
        if let Err(error) = takeover.restore(*client) {
            if let Some((base_url, local_credential)) = &gateway {
                for restored_client in restored.into_iter().rev() {
                    if let Some((_, model, context1m)) = configs
                        .iter()
                        .find(|(client, _, _)| *client == restored_client)
                    {
                        if let Err(takeover_error) = takeover.take_over(
                            restored_client,
                            base_url,
                            local_credential,
                            model.as_deref(),
                            *context1m,
                        ) {
                            log::error!(
                                "Failed to roll back gateway stop for {}: {}",
                                restored_client.display_name(),
                                takeover_error
                            );
                        }
                    }
                }
            }
            return Err(CommandError::infrastructure(error.to_string()));
        }
        restored.push(*client);
    }

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

#[tauri::command]
#[specta::specta]
pub async fn recover_gateway_requests(
    client: GatewayClientDto,
    state: State<'_, GatewayState>,
) -> Result<GatewayStatusDto, CommandError> {
    let gateway_client: GatewayClient = client.into();
    let _operation = state.operation_lock(gateway_client).lock().await;
    let upstream = state
        .bindings
        .read()
        .await
        .get(&gateway_client)
        .cloned()
        .ok_or_else(|| CommandError::validation("The Agent has no gateway binding."))?;
    state.runtime.recover(gateway_client, &upstream.id).await;
    Ok(status_of(&state).await)
}

#[tauri::command]
#[specta::specta]
pub async fn clear_gateway_statistics(
    client: GatewayClientDto,
    state: State<'_, GatewayState>,
) -> Result<GatewayStatusDto, CommandError> {
    let gateway_client: GatewayClient = client.into();
    let _operation = state.operation_lock(gateway_client).lock().await;
    let upstream = state
        .bindings
        .read()
        .await
        .get(&gateway_client)
        .cloned()
        .ok_or_else(|| CommandError::validation("The Agent has no gateway binding."))?;
    state
        .runtime
        .clear_statistics(gateway_client, &upstream.id)
        .await;
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
