use serde::{Deserialize, Serialize};
use std::fs;
use tauri::State;

use super::gateway_fingerprint::{claude_fingerprint, codex_fingerprint, AgentConfigFingerprint};
use super::gateway_lifecycle::{apply_gateway_config, clear_gateway_config, takeover_for};
use super::gateway_mapping::{
    access_source_of, binding_route, binding_source_input, client_for_agent, source_from_access,
    upstream_kind_str,
};
use super::gateway_persistence::{
    app_data_dir, read_bindings, restore_binding_memory, rollback_binding_change, write_bindings,
    BindingMeta, IsolatedTarget, RUNTIME_FILENAME,
};
use super::gateway_preview::{fingerprint_from_preview, preview_files_to_dto};
use super::gateway_profiles::{
    abbreviate_home, list_claude_profiles, list_codex_config_files, match_profile_account,
};
pub use super::gateway_runtime::{
    clear_gateway_statistics, get_gateway_status, recover_gateway_requests,
};
pub use super::gateway_state::GatewayState;
use super::gateway_status::status_of;
use crate::presentation::error::CommandError;
use crate::presentation::state::{Repositories, Services};
use neuradock_domain::independent_key::IndependentKeyId;
use neuradock_domain::shared::{AccountId, ProviderId};
use neuradock_infrastructure::gateway::{
    start_gateway_with_runtime, CircuitState, GatewayClient, GatewayRoute, GatewayUpstream,
    UpstreamKind,
};

use crate::application::services::{
    ClaudeConfigService, CodexConfigService, GatewayBindingConfigInput, GatewayBindingService,
    GatewayBindingSourceInput, GatewayResolveError, GatewayResolverDeps,
};
use crate::presentation::commands::agent_access::{
    AccessMethodDto, AgentAccessDto, AgentAccessSourceDto, AgentAccessState, AgentIdDto,
};
use crate::presentation::commands::token::ensure_account_token_usable;

type GatewayStopConfig = (
    GatewayClient,
    String,
    Option<String>,
    bool,
    Option<IsolatedTarget>,
);

/// Per-client runtime metadata plus everything needed to persist and rebuild
/// the binding across restarts (the real key is never stored; it is re-resolved
/// from the source).
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
    pub profile: String,
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
    #[serde(default)]
    pub isolated: bool,
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
    Missing,
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
    pub codex_configs: Vec<CodexConfigFileDto>,
    pub profiles: Vec<AgentProfileDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct AgentProfileDto {
    pub kind: String,
    pub slug: Option<String>,
    pub path: String,
    pub exists: bool,
    pub parse_ok: bool,
    pub configured: bool,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub context1m: bool,
    pub credential_present: bool,
    pub managed: bool,
    pub account_label: Option<String>,
    pub gateway_enabled: bool,
    pub method: Option<AccessMethodDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CodexConfigFileDto {
    pub kind: String,
    pub slug: Option<String>,
    pub path: String,
    pub current: bool,
    pub exists: bool,
    pub parse_ok: bool,
    pub configured: bool,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub context1m: bool,
    pub credential_present: bool,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GatewayBindingConfigDto {
    pub upstream_kind: UpstreamKindDto,
    pub model: Option<String>,
    #[serde(default, rename = "context1m")]
    pub context_1m: bool,
    pub impersonate: Option<bool>,
    #[serde(default)]
    pub isolated: bool,
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn set_gateway_profile_enabled(
    client: GatewayClientDto,
    profile: String,
    enabled: bool,
    app: tauri::AppHandle,
    state: State<'_, GatewayState>,
    access: State<'_, AgentAccessState>,
    services: State<'_, Services>,
    repositories: State<'_, Repositories>,
) -> Result<GatewayStatusDto, CommandError> {
    let gateway_client: GatewayClient = client.into();
    let _operation = state.operation_lock(gateway_client).lock().await;
    let route = GatewayRoute {
        client: gateway_client,
        profile,
    };
    if !enabled {
        let old_upstream = state.bindings.read().await.get(&route).cloned();
        let old_meta = state.meta.lock().await.get(&route).cloned();
        state.bindings.write().await.remove(&route);
        if let Some(meta) = state.meta.lock().await.get_mut(&route) {
            meta.enabled = false;
        }
        if let Err(error) = write_bindings(&app, &state).await {
            restore_binding_memory(&state, route, old_upstream, old_meta).await;
            return Err(error);
        }
    } else {
        let meta = state
            .meta
            .lock()
            .await
            .get(&route)
            .cloned()
            .ok_or_else(|| {
                CommandError::validation(
                    "This Profile has no saved Gateway binding; edit and save it first.",
                )
            })?;
        let effective_model = GatewayBindingService::effective_model(meta.model.as_deref());
        let (upstream, _) = resolve_upstream(
            &binding_source_input(&meta.source),
            meta.kind.into(),
            effective_model.as_deref(),
            meta.impersonate,
            &services,
            &repositories,
        )
        .await?;
        let old_upstream = state.bindings.read().await.get(&route).cloned();
        let old_meta = state.meta.lock().await.get(&route).cloned();
        state.bindings.write().await.insert(route.clone(), upstream);
        if let Some(saved) = state.meta.lock().await.get_mut(&route) {
            saved.enabled = true;
        }
        if let Err(error) = write_bindings(&app, &state).await {
            restore_binding_memory(&state, route, old_upstream, old_meta).await;
            return Err(error);
        }
        if state.server.lock().await.is_none() {
            if let Err(error) =
                start_gateway_server(app.clone(), state.clone(), access.clone()).await
            {
                restore_binding_memory(
                    &state,
                    route.clone(),
                    old_upstream.clone(),
                    old_meta.clone(),
                )
                .await;
                if let Err(write_error) = write_bindings(&app, &state).await {
                    log::error!("Failed to persist profile rollback: {}", write_error);
                }
                return Err(error);
            }
        } else {
            let server = state.server.lock().await;
            let handle = server
                .as_ref()
                .ok_or_else(|| CommandError::infrastructure("Gateway stopped unexpectedly"))?;
            let profile_base = format!("{}/profiles/{}", handle.base_url(), route.profile);
            let takeover = match takeover_for(&app) {
                Ok(takeover) => takeover,
                Err(error) => {
                    drop(server);
                    restore_binding_memory(
                        &state,
                        route.clone(),
                        old_upstream.clone(),
                        old_meta.clone(),
                    )
                    .await;
                    if let Err(write_error) = write_bindings(&app, &state).await {
                        log::error!("Failed to persist profile rollback: {}", write_error);
                    }
                    return Err(error);
                }
            };
            if let Err(error) = apply_gateway_config(
                &takeover,
                route.client,
                &profile_base,
                handle.local_credential(),
                meta.model.as_deref(),
                meta.context1m,
                meta.isolated.as_ref(),
            ) {
                restore_binding_memory(
                    &state,
                    route.clone(),
                    old_upstream.clone(),
                    old_meta.clone(),
                )
                .await;
                if let Err(write_error) = write_bindings(&app, &state).await {
                    log::error!("Failed to persist profile rollback: {}", write_error);
                }
                return Err(error);
            }
        }
    }
    Ok(status_of(&state).await)
}

/// Resolve a binding source into the upstream the gateway forwards to. This is
/// the single exhaustive place key shapes are read; adding a source variant
/// fails to compile until handled.
async fn resolve_upstream(
    source: &GatewayBindingSourceInput,
    kind: UpstreamKind,
    model: Option<&str>,
    impersonate: bool,
    services: &Services,
    repositories: &Repositories,
) -> Result<(GatewayUpstream, String), CommandError> {
    let resolved = GatewayBindingService::resolve_upstream(
        source,
        model,
        impersonate,
        GatewayResolverDeps {
            accounts: repositories.account.as_ref(),
            providers: repositories.provider.as_ref(),
            independent_keys: repositories.independent_key.as_ref(),
            tokens: services.token.as_ref(),
        },
    )
    .await
    .map_err(map_gateway_resolve_error)?;
    let label = resolved.label.clone();
    Ok((
        GatewayUpstream {
            id: resolved.id,
            label: resolved.label,
            base_url: resolved.base_url,
            api_key: resolved.api_key,
            kind,
            model: resolved.model,
            impersonate: resolved.impersonate,
        },
        label,
    ))
}

fn map_gateway_resolve_error(error: GatewayResolveError) -> CommandError {
    match error {
        GatewayResolveError::AccountNotFound => CommandError::not_found("Account not found"),
        GatewayResolveError::TokenNotFound => CommandError::not_found("Token not found"),
        GatewayResolveError::ProviderNotFound => CommandError::not_found("Provider not found"),
        GatewayResolveError::ProviderMismatch => CommandError::validation(error.to_string()),
        GatewayResolveError::IndependentKeyNotFound(id) => {
            CommandError::not_found(format!("Key with ID {id} not found"))
        }
        GatewayResolveError::IndependentKeyInactive => CommandError::validation(error.to_string()),
        GatewayResolveError::TokenUnusable(_) => CommandError::validation(error.to_string()),
        GatewayResolveError::Domain(error) => error.into(),
        GatewayResolveError::Service(message) => CommandError::infrastructure(message),
    }
}

/// Derive the isolated-file identity (slug + display name) for a binding, so
/// the isolated Codex config is named after the real relay provider even though
/// its `base_url` points at the local gateway. Only the label/name is read here;
/// no key material is touched.
async fn isolated_target_for(
    client: GatewayClientDto,
    source: &GatewaySourceDto,
    repositories: &Repositories,
) -> Result<IsolatedTarget, CommandError> {
    match source {
        GatewaySourceDto::AccountToken {
            account_id: _,
            provider_id,
            token_id: _,
            base_url: _,
        } => {
            let provider = repositories
                .provider
                .find_by_id(&ProviderId::from_string(provider_id))
                .await
                .map_err(CommandError::from)?;
            let display = provider
                .as_ref()
                .map(|p| p.name().to_string())
                .unwrap_or_else(|| provider_id.clone());
            Ok(IsolatedTarget {
                slug: if client == GatewayClientDto::ClaudeCode {
                    display.clone()
                } else {
                    provider_id.clone()
                },
                display,
            })
        }
        GatewaySourceDto::IndependentKey { key_id } => {
            let key = repositories
                .independent_key
                .find_by_id(&IndependentKeyId::new(*key_id))
                .await
                .map_err(CommandError::from)?
                .ok_or_else(|| CommandError::not_found("Key not found"))?;
            // Slug from the key's base_url host; display from the key name.
            let slug = url::Url::parse(key.base_url())
                .ok()
                .and_then(|u| u.host_str().map(|h| h.to_string()))
                .unwrap_or_else(|| key.name().to_string());
            Ok(IsolatedTarget {
                slug,
                display: key.name().to_string(),
            })
        }
    }
}

/// Read the config a tool is actually running with. `isolated_slug` selects the
/// isolated Codex config (`<slug>.config.toml`) instead of the official
/// config.toml/auth.json, so a binding is compared against the file it wrote.
pub(super) fn read_agent_fingerprint(
    client: GatewayClientDto,
    isolated_slug: Option<&str>,
) -> Result<AgentConfigFingerprint, CommandError> {
    match client {
        GatewayClientDto::ClaudeCode => {
            let service = ClaudeConfigService::new();
            let path = if let Some(slug) = isolated_slug {
                service.isolated_path(slug).map_err(CommandError::from)?
            } else {
                ClaudeConfigService::managed_path().map_err(CommandError::from)?
            };
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
            let (config_path, auth_path) = if let Some(slug) = isolated_slug {
                (
                    CodexConfigService::isolated_path(slug).map_err(CommandError::from)?,
                    None,
                )
            } else {
                let (config_path, auth_path) =
                    CodexConfigService::managed_paths().map_err(CommandError::from)?;
                (config_path, Some(auth_path))
            };
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
            let auth_content = match auth_path.as_ref() {
                Some(path) if path.exists() => fs::read_to_string(path).map_err(|error| {
                    CommandError::infrastructure(format!(
                        "Failed to read {}: {error}",
                        path.display()
                    ))
                })?,
                _ => String::new(),
            };
            let mut paths = vec![config_path.clone()];
            if let Some(path) = auth_path {
                paths.push(path);
            }
            let fallback_paths = paths.clone();
            Ok(
                codex_fingerprint(&config_content, &auth_content, paths).unwrap_or_else(|_| {
                    AgentConfigFingerprint {
                        parse_ok: false,
                        paths: fallback_paths,
                        ..Default::default()
                    }
                }),
            )
        }
    }
}

/* fn fingerprint_from_preview_legacy(
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
            let auth = files.iter().find(|(_, format, _, _)| *format == "json");
            let auth_content = auth.map(|file| file.3.as_str()).unwrap_or("");
            let paths = files.iter().map(|file| file.0.clone()).collect();
            codex_fingerprint(&config.3, auth_content, paths).map_err(CommandError::from)
        }
    }
} */

async fn expected_fingerprint(
    record: &AgentAccessDto,
    gateway_base_url: Option<&str>,
    gateway_credential: Option<&str>,
    services: &Services,
    repositories: &Repositories,
) -> Result<Option<AgentConfigFingerprint>, CommandError> {
    let client = client_for_agent(record.agent);
    let context1m = record.context1m.unwrap_or(false);
    let isolated_slug = record.isolated_slug.as_deref();
    let files = match record.method {
        AccessMethodDto::Gateway => {
            let (Some(base_url), Some(credential)) = (gateway_base_url, gateway_credential) else {
                return Ok(None);
            };
            match isolated_slug {
                // Isolated gateway bindings write a file named after the relay
                // whose base_url is the gateway; rebuild that exact shape.
                Some(slug) => match client {
                    GatewayClientDto::Codex => services
                        .codex_config
                        .preview_isolated_gateway(
                            slug,
                            &record.label,
                            base_url,
                            credential,
                            record.model.as_deref(),
                            context1m,
                        )
                        .map_err(CommandError::from)?,
                    GatewayClientDto::ClaudeCode => services
                        .claude_config
                        .preview_isolated_with_key(
                            slug,
                            credential,
                            base_url,
                            record.model.as_deref(),
                            context1m,
                        )
                        .map(|(path, current, next)| vec![(path, "json", current, next)])
                        .map_err(CommandError::from)?,
                },
                None => preview_with_key(
                    client,
                    credential,
                    base_url,
                    record.model.as_deref(),
                    context1m,
                    false,
                    services,
                )?,
            }
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
                isolated_slug.is_some(),
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
    app: tauri::AppHandle,
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
        let mut profile_records = access.list(agent);
        for profile_record in &mut profile_records {
            if profile_record.account_label.is_none() {
                if let Some(AgentAccessSourceDto::AccountToken { account_id, .. }) =
                    &profile_record.source
                {
                    profile_record.account_label = repositories
                        .account
                        .find_by_id(&AccountId::from_string(account_id))
                        .await
                        .map_err(CommandError::from)?
                        .map(|account| account.name().to_string());
                }
            }
        }
        let mut record = access.get(agent);
        let mut actual = read_agent_fingerprint(
            client,
            record
                .as_ref()
                .and_then(|record| record.isolated_slug.as_deref()),
        )?;
        let mut expected = match record.as_ref() {
            Some(record) => {
                expected_fingerprint(
                    record,
                    gateway.as_ref().map(|(base_url, _)| base_url.as_str()),
                    gateway.as_ref().map(|(_, credential)| credential.as_str()),
                    &services,
                    &repositories,
                )
                .await?
            }
            None => None,
        };

        // Older direct-isolated records did not persist isolated_slug. Repair
        // them only when the official config does not match and the inferred
        // isolated file matches exactly, avoiding ambiguous migrations.
        let current_matches = expected.as_ref().is_some_and(|expected| {
            actual.parse_ok && actual.managed_values == expected.managed_values
        });
        if agent == AgentIdDto::Codex && !current_matches {
            if let Some(current) = record.as_ref().filter(|record| {
                record.method == AccessMethodDto::Direct
                    && record.isolated_slug.is_none()
                    && record.source.is_some()
            }) {
                let source = source_from_access(current.source.as_ref().expect("checked above"));
                let target = isolated_target_for(client, &source, &repositories).await?;
                let mut candidate = current.clone();
                candidate.isolated_slug = Some(target.slug.clone());
                let candidate_actual = read_agent_fingerprint(client, Some(&target.slug))?;
                let candidate_expected = expected_fingerprint(
                    &candidate,
                    gateway.as_ref().map(|(base_url, _)| base_url.as_str()),
                    gateway.as_ref().map(|(_, credential)| credential.as_str()),
                    &services,
                    &repositories,
                )
                .await?;
                let candidate_matches = candidate_actual.exists
                    && candidate_expected.as_ref().is_some_and(|expected| {
                        candidate_actual.parse_ok
                            && candidate_actual.managed_values == expected.managed_values
                    });
                if candidate_matches {
                    access.set(&app, candidate.clone())?;
                    record = Some(candidate);
                    actual = candidate_actual;
                    expected = candidate_expected;
                }
            }
        }

        let state_kind = if record.is_some() {
            if !actual.exists {
                AgentConfigurationKindDto::Missing
            } else {
                match expected.as_ref() {
                    None => AgentConfigurationKindDto::Managed,
                    Some(expected)
                        if actual.parse_ok && actual.managed_values == expected.managed_values =>
                    {
                        AgentConfigurationKindDto::Managed
                    }
                    _ => AgentConfigurationKindDto::Changed,
                }
            }
        } else if actual.parse_ok && actual.configured {
            AgentConfigurationKindDto::Manual
        } else {
            AgentConfigurationKindDto::Unconfigured
        };

        let codex_configs = if agent == AgentIdDto::Codex {
            list_codex_config_files(
                record.is_some(),
                record
                    .as_ref()
                    .and_then(|record| record.isolated_slug.as_deref()),
            )?
        } else {
            Vec::new()
        };
        let profiles = if agent == AgentIdDto::Codex {
            codex_configs
                .iter()
                .map(|item| AgentProfileDto {
                    kind: item.kind.clone(),
                    slug: item.slug.clone(),
                    path: item.path.clone(),
                    exists: item.exists,
                    parse_ok: item.parse_ok,
                    configured: item.configured,
                    endpoint: item.endpoint.clone(),
                    model: item.model.clone(),
                    context1m: item.context1m,
                    credential_present: item.credential_present,
                    managed: true,
                    account_label: profile_records
                        .iter()
                        .find(|record| {
                            if item.kind == "official" {
                                record.isolated_slug.is_none()
                            } else {
                                item.slug.as_deref() == record.isolated_slug.as_deref()
                            }
                        })
                        .and_then(|record| record.account_label.clone()),
                    gateway_enabled: profile_records.iter().any(|record| {
                        record.method == AccessMethodDto::Gateway
                            && if item.kind == "official" {
                                record.isolated_slug.is_none()
                            } else {
                                item.slug.as_deref() == record.isolated_slug.as_deref()
                            }
                    }),
                    method: profile_records
                        .iter()
                        .find(|record| {
                            if item.kind == "official" {
                                record.isolated_slug.is_none()
                            } else {
                                item.slug.as_deref() == record.isolated_slug.as_deref()
                            }
                        })
                        .map(|record| record.method),
                })
                .collect()
        } else {
            let mut profiles = list_claude_profiles()?;
            for profile in &mut profiles {
                profile.managed = true;
                profile.account_label = profile_records
                    .iter()
                    .find(|record| {
                        if profile.kind == "official" {
                            record.isolated_slug.is_none()
                        } else {
                            profile.slug.as_deref() == record.isolated_slug.as_deref()
                        }
                    })
                    .and_then(|record| record.account_label.clone());
                profile.gateway_enabled = profile_records.iter().any(|record| {
                    record.method == AccessMethodDto::Gateway
                        && if profile.kind == "official" {
                            record.isolated_slug.is_none()
                        } else {
                            profile.slug.as_deref() == record.isolated_slug.as_deref()
                        }
                });
                profile.method = profile_records
                    .iter()
                    .find(|record| {
                        if profile.kind == "official" {
                            record.isolated_slug.is_none()
                        } else {
                            profile.slug.as_deref() == record.isolated_slug.as_deref()
                        }
                    })
                    .map(|record| record.method);
            }
            profiles
        };
        let mut profiles = profiles;
        for profile in &mut profiles {
            if profile.account_label.is_none() && profile.exists && profile.parse_ok {
                profile.account_label =
                    match_profile_account(client, profile, &services, &repositories).await?;
            }
        }

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
            codex_configs,
            profiles,
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
                .unwrap_or("sk-ndg-preview-credential");
            preview_with_key(
                request.client,
                preview_credential,
                preview_base_url,
                request.model.as_deref(),
                request.context_1m,
                request.isolated,
                &services,
            )?
        }
        AccessMethodDto::Direct => {
            preview_direct(
                request.client,
                &request.source,
                request.model.as_deref(),
                request.context_1m,
                request.isolated,
                &services,
                &repositories,
            )
            .await?
        }
    };

    let files = preview_files_to_dto(
        raw_files,
        request.method == AccessMethodDto::Gateway,
        gateway_running,
    )
    .into_iter()
    .map(|mut file| {
        file.path = abbreviate_home(std::path::Path::new(&file.path));
        file
    })
    .collect();

    Ok(AgentConfigPreviewDto {
        files,
        takeover_active,
        gateway_running,
    })
}

pub(super) type RawPreviewFile = (std::path::PathBuf, &'static str, String, String);

fn preview_with_key(
    client: GatewayClientDto,
    api_key: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
    isolated: bool,
    services: &Services,
) -> Result<Vec<RawPreviewFile>, CommandError> {
    match client {
        GatewayClientDto::Codex => {
            if isolated {
                services
                    .codex_config
                    .preview_isolated_with_key(api_key, base_url, model, context_1m)
                    .map_err(CommandError::from)
            } else {
                services
                    .codex_config
                    .preview_global_with_key(api_key, base_url, model, context_1m)
                    .map_err(CommandError::from)
            }
        }
        GatewayClientDto::ClaudeCode => {
            let preview = if isolated {
                let slug = url::Url::parse(base_url)
                    .ok()
                    .and_then(|url| url.host_str().map(str::to_string))
                    .unwrap_or_else(|| "profile".to_string());
                services
                    .claude_config
                    .preview_isolated_with_key(&slug, api_key, base_url, model, context_1m)
            } else {
                services
                    .claude_config
                    .preview_global_with_key(api_key, base_url, model, context_1m)
            };
            preview
                .map(|(path, current, next)| vec![(path, "json", current, next)])
                .map_err(CommandError::from)
        }
    }
}

async fn preview_direct(
    client: GatewayClientDto,
    source: &GatewaySourceDto,
    model: Option<&str>,
    context_1m: bool,
    isolated: bool,
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
                GatewayClientDto::Codex => {
                    if isolated {
                        services
                            .codex_config
                            .preview_isolated(
                                token,
                                provider.id().as_str(),
                                provider.name(),
                                base_url,
                                model,
                                context_1m,
                            )
                            .map_err(CommandError::from)
                    } else {
                        services
                            .codex_config
                            .preview_global(
                                token,
                                provider.id().as_str(),
                                provider.name(),
                                base_url,
                                model,
                                context_1m,
                            )
                            .map_err(CommandError::from)
                    }
                }
                GatewayClientDto::ClaudeCode => {
                    let preview = if isolated {
                        services.claude_config.preview_isolated_with_key(
                            provider.name(),
                            token.key(),
                            base_url,
                            model,
                            context_1m,
                        )
                    } else {
                        services.claude_config.preview_global_with_key(
                            token.key(),
                            base_url,
                            model,
                            context_1m,
                        )
                    };
                    preview
                        .map(|(path, current, next)| vec![(path, "json", current, next)])
                        .map_err(CommandError::from)
                }
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
                isolated,
                services,
            )
        }
    }
}

/// Bind a client (Codex / Claude Code) to an upstream relay key. If the
/// gateway is running, the client's tool config is (re)taken over immediately.
#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
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
    let upstream_kind = config.upstream_kind;
    let normalized = GatewayBindingService::normalize_config(GatewayBindingConfigInput {
        model: config.model,
        context1m: config.context_1m,
        impersonate: config.impersonate,
        isolated: config.isolated,
    })
    .map_err(|error| CommandError::validation(error.to_string()))?;
    let model = normalized.model;
    let context1m = normalized.context1m;
    let impersonate = normalized.impersonate;
    let isolated_requested = normalized.isolated;
    let effective_model = GatewayBindingService::effective_model(model.as_deref());
    let (upstream, label) = resolve_upstream(
        &binding_source_input(&source),
        upstream_kind.into(),
        effective_model.as_deref(),
        impersonate,
        &services,
        &repositories,
    )
    .await?;
    let isolated = if isolated_requested {
        Some(isolated_target_for(client, &source, &repositories).await?)
    } else {
        None
    };
    let gateway_client: GatewayClient = client.into();
    let agent: AgentIdDto = client.into();
    let _operation = state.operation_lock(gateway_client).lock().await;
    let access_source = access_source_of(&source);
    let account_label = match &access_source {
        AgentAccessSourceDto::AccountToken { account_id, .. } => repositories
            .account
            .find_by_id(&AccountId::from_string(account_id))
            .await
            .map_err(CommandError::from)?
            .map(|account| account.name().to_string()),
        AgentAccessSourceDto::IndependentKey { .. } => None,
    };
    let route = binding_route(
        gateway_client,
        isolated.as_ref().map(|target| target.slug.as_str()),
    );
    let old_upstream = state.bindings.read().await.get(&route).cloned();
    let old_meta = state.meta.lock().await.get(&route).cloned();
    let old_access = access.get(agent);
    let new_meta = BindingMeta {
        label: label.clone(),
        source,
        kind: upstream_kind,
        model: model.clone(),
        context1m,
        impersonate,
        isolated: isolated.clone(),
        enabled: true,
    };
    let new_access = AgentAccessDto {
        agent,
        method: AccessMethodDto::Gateway,
        label,
        account_label,
        model: model.clone(),
        upstream_kind: Some(upstream_kind_str(upstream_kind)),
        impersonate: Some(impersonate),
        source: Some(access_source),
        context1m: Some(context1m),
        isolated_slug: isolated.as_ref().map(|t| t.slug.clone()),
    };

    state.bindings.write().await.insert(route.clone(), upstream);
    state.meta.lock().await.insert(route.clone(), new_meta);

    if let Err(error) = write_bindings(&app, &state).await {
        restore_binding_memory(&state, route.clone(), old_upstream, old_meta).await;
        return Err(error);
    }

    // Mirror into the access record so the status card shows a consistent view.
    if let Err(error) = access.set(&app, new_access) {
        rollback_binding_change(
            &app,
            &state,
            &access,
            route.clone(),
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
                    route.clone(),
                    agent,
                    old_upstream,
                    old_meta,
                    old_access,
                )
                .await;
                return Err(error);
            }
        };
        if let Err(error) = apply_gateway_config(
            &takeover,
            gateway_client,
            &format!("{}/profiles/{}", handle.base_url(), route.profile),
            handle.local_credential(),
            model.as_deref(),
            context1m,
            isolated.as_ref(),
        ) {
            drop(server);
            rollback_binding_change(
                &app,
                &state,
                &access,
                route.clone(),
                agent,
                old_upstream,
                old_meta,
                old_access,
            )
            .await;
            return Err(error);
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
    services: State<'_, Services>,
) -> Result<GatewayStatusDto, CommandError> {
    let gateway_client: GatewayClient = client.into();
    let agent: AgentIdDto = client.into();
    let _operation = state.operation_lock(gateway_client).lock().await;
    let old_access = access.get(agent);
    let route = binding_route(
        gateway_client,
        old_access
            .as_ref()
            .and_then(|record| record.isolated_slug.as_deref()),
    );
    let old_upstream = state.bindings.read().await.get(&route).cloned();
    let old_meta = state.meta.lock().await.get(&route).cloned();

    state.bindings.write().await.remove(&route);
    state.meta.lock().await.remove(&route);

    if let Err(error) = write_bindings(&app, &state).await {
        restore_binding_memory(&state, route.clone(), old_upstream, old_meta).await;
        return Err(error);
    }
    if let Err(error) = access.remove(&app, agent) {
        rollback_binding_change(
            &app,
            &state,
            &access,
            route.clone(),
            agent,
            old_upstream,
            old_meta,
            old_access,
        )
        .await;
        return Err(error);
    }

    if let Some(slug) = old_access
        .as_ref()
        .and_then(|record| record.isolated_slug.as_deref())
    {
        match gateway_client {
            GatewayClient::Codex => services.codex_config.clear_isolated(slug),
            GatewayClient::ClaudeCode => services.claude_config.clear_isolated(slug),
        }
        .map_err(CommandError::from)?;
    } else {
        let takeover = match takeover_for(&app) {
            Ok(takeover) => takeover,
            Err(error) => {
                rollback_binding_change(
                    &app,
                    &state,
                    &access,
                    route.clone(),
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
                route,
                agent,
                old_upstream,
                old_meta,
                old_access,
            )
            .await;
            return Err(CommandError::infrastructure(error.to_string()));
        }
    }
    state.runtime.reset(gateway_client).await;

    Ok(status_of(&state).await)
}

/// Drop a client's saved gateway binding without touching its access record.
/// Called when the agent is reconfigured for direct write, so a later gateway
/// start does not take the client over.
pub async fn drop_gateway_binding_for_direct(
    app: &tauri::AppHandle,
    state: &GatewayState,
    client: GatewayClient,
) -> Result<(), CommandError> {
    let _operation = state.operation_lock(client).lock().await;
    let old_upstream = state
        .bindings
        .read()
        .await
        .get(&GatewayRoute::default(client))
        .cloned();
    let old_meta = state
        .meta
        .lock()
        .await
        .get(&GatewayRoute::default(client))
        .cloned();
    if old_upstream.is_none() && old_meta.is_none() {
        return Ok(());
    }
    state
        .bindings
        .write()
        .await
        .remove(&GatewayRoute::default(client));
    state
        .meta
        .lock()
        .await
        .remove(&GatewayRoute::default(client));
    if let Err(error) = write_bindings(app, state).await {
        restore_binding_memory(state, GatewayRoute::default(client), old_upstream, old_meta).await;
        return Err(error);
    }

    // Retire the takeover record too. The caller has just written the user's
    // real key directly into the tool config, so we must NOT call
    // `takeover.restore()` here -- that would overwrite the new config with the
    // pre-takeover snapshot. But leaving the record is equally broken:
    // `stop_gateway_server` and the exit handler both walk it and would revert
    // the user's direct config on every shutdown, silently and with no error.
    // Discarding drops the snapshot without touching the live files.
    match takeover_for(app) {
        Ok(takeover) => {
            if let Err(error) = takeover.discard(client) {
                restore_binding_memory(
                    state,
                    GatewayRoute::default(client),
                    old_upstream,
                    old_meta,
                )
                .await;
                return Err(CommandError::infrastructure(error.to_string()));
            }
        }
        Err(error) => {
            restore_binding_memory(state, GatewayRoute::default(client), old_upstream, old_meta)
                .await;
            return Err(error);
        }
    }

    state.runtime.reset(client).await;
    Ok(())
}

/// Persist the current bindings (references only, never keys) to app data so
/// they survive a restart.
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
    let Ok(dir) = app_data_dir(app) else {
        return;
    };
    state
        .runtime
        .configure_persistence(dir.join(RUNTIME_FILENAME))
        .await;
    let saved = match read_bindings(&dir) {
        Ok(bindings) => bindings,
        Err(_) => return,
    };

    for binding in saved {
        // The user may have switched this agent to direct write since the
        // binding was saved; their choice wins over the stale binding.
        let binding_slug = binding.isolated.as_ref().map(|target| target.slug.as_str());
        if access.list(binding.client.into()).iter().any(|record| {
            record.method == AccessMethodDto::Direct
                && record.isolated_slug.as_deref() == binding_slug
        }) {
            log::info!(
                "Skipping saved gateway binding for {:?}: agent is set to direct access",
                binding.client
            );
            continue;
        }
        let effective_model = GatewayBindingService::effective_model(binding.model.as_deref());
        match resolve_upstream(
            &binding_source_input(&binding.source),
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
                let route = binding_route(
                    client,
                    binding.isolated.as_ref().map(|target| target.slug.as_str()),
                );
                if binding.enabled {
                    state.bindings.write().await.insert(route.clone(), upstream);
                }
                state.meta.lock().await.insert(
                    route,
                    BindingMeta {
                        label: label.clone(),
                        source: binding.source,
                        kind: binding.kind,
                        model: binding.model.clone(),
                        context1m: binding.context1m,
                        impersonate: binding.impersonate,
                        isolated: binding.isolated.clone(),
                        enabled: binding.enabled,
                    },
                );
                if !binding.enabled {
                    if let Err(error) = access.remove_profile(
                        app,
                        binding.client.into(),
                        binding.isolated.as_ref().map(|target| target.slug.as_str()),
                    ) {
                        log::warn!(
                            "Failed to remove disabled access record for {:?}: {}",
                            binding.client,
                            error
                        );
                    }
                } else if let Err(error) = access.set(
                    app,
                    AgentAccessDto {
                        agent: binding.client.into(),
                        method: AccessMethodDto::Gateway,
                        label,
                        account_label: None,
                        model: binding.model,
                        upstream_kind: Some(upstream_kind_str(binding.kind)),
                        impersonate: Some(binding.impersonate),
                        source: Some(access_source),
                        context1m: Some(binding.context1m),
                        isolated_slug: binding.isolated.as_ref().map(|t| t.slug.clone()),
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
                if let Err(remove_error) = access.remove_profile(
                    app,
                    binding.client.into(),
                    binding.isolated.as_ref().map(|target| target.slug.as_str()),
                ) {
                    log::warn!(
                        "Failed to remove stale access profile for {:?}: {}",
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

/// Start the gateway server and take over every gateway-bound client's tool
/// config. Clients whose access record says direct write are left untouched.
#[tauri::command]
#[specta::specta]
pub async fn start_gateway_server(
    app: tauri::AppHandle,
    state: State<'_, GatewayState>,
    access: State<'_, AgentAccessState>,
) -> Result<GatewayStatusDto, CommandError> {
    let _codex_operation = state.codex_operation.lock().await;
    let _claude_operation = state.claude_operation.lock().await;
    let takeover_configs: Vec<(GatewayClient, Option<String>, bool, Option<IsolatedTarget>)> =
        state
            .meta
            .lock()
            .await
            .iter()
            .filter(|(client, meta)| {
                meta.enabled
                    && access
                        .get(GatewayClientDto::from(client.client).into())
                        .is_none_or(|record| record.method == AccessMethodDto::Gateway)
            })
            .map(|(client, meta)| {
                (
                    client.client,
                    meta.model.clone(),
                    meta.context1m,
                    meta.isolated.clone(),
                )
            })
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

    let mut taken_over: Vec<(GatewayClient, Option<IsolatedTarget>)> = Vec::new();
    for (client, model, context1m, isolated) in takeover_configs {
        let profile = isolated
            .as_ref()
            .map(|target| target.slug.as_str())
            .unwrap_or("default");
        if let Err(error) = apply_gateway_config(
            &takeover,
            client,
            &format!("{base_url}/profiles/{profile}"),
            &local_credential,
            model.as_deref(),
            context1m,
            isolated.as_ref(),
        ) {
            for (restored_client, isolated) in taken_over.into_iter().rev() {
                if let Some(target) = isolated.as_ref() {
                    if let Err(clear_error) = clear_gateway_config(restored_client, Some(target)) {
                        log::error!(
                            "Failed to clear gateway startup profile for {}: {}",
                            restored_client.display_name(),
                            clear_error
                        );
                    }
                } else if let Err(restore_error) = takeover.restore(restored_client) {
                    log::error!(
                        "Failed to roll back gateway startup for {}: {}",
                        restored_client.display_name(),
                        restore_error
                    );
                }
            }
            if let Err(clear_error) = clear_gateway_config(client, isolated.as_ref()) {
                log::error!("Failed to clear failed gateway profile: {}", clear_error);
            }
            if let Some(handle) = state.server.lock().await.take() {
                handle.shutdown().await;
            }
            return Err(CommandError::infrastructure(error.to_string()));
        }
        taken_over.push((client, isolated));
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
    let configs: Vec<GatewayStopConfig> = state
        .meta
        .lock()
        .await
        .iter()
        .filter(|(_, meta)| meta.enabled)
        .map(|(client, meta)| {
            (
                client.client,
                client.profile.clone(),
                meta.model.clone(),
                meta.context1m,
                meta.isolated.clone(),
            )
        })
        .collect();
    let mut restored = Vec::new();
    for (client, _, _, _, isolated) in &configs {
        if isolated.is_none() && !takeover.is_active_for(*client) {
            continue;
        }
        let restore_result = if isolated.is_some() {
            clear_gateway_config(*client, isolated.as_ref())
        } else {
            takeover
                .restore(*client)
                .map_err(|error| CommandError::infrastructure(error.to_string()))
        };
        if let Err(error) = restore_result {
            if let Some((base_url, local_credential)) = &gateway {
                for restored_client in restored.into_iter().rev() {
                    if let Some((_, profile, model, context1m, isolated)) = configs
                        .iter()
                        .find(|(client, _, _, _, _)| *client == restored_client)
                    {
                        if let Err(takeover_error) = apply_gateway_config(
                            &takeover,
                            restored_client,
                            &format!("{base_url}/profiles/{profile}"),
                            local_credential,
                            model.as_deref(),
                            *context1m,
                            isolated.as_ref(),
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
            return Err(error);
        }
        restored.push(*client);
    }

    if let Some(handle) = state.server.lock().await.take() {
        handle.shutdown().await;
    }

    Ok(status_of(&state).await)
}

/// Report gateway state for the sidebar switch and panel.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::commands::gateway_persistence::PersistedBinding;
    use crate::presentation::commands::gateway_redaction::redact_content;
    use std::path::PathBuf;

    #[test]
    fn gateway_upstream_model_never_carries_1m_suffix() {
        assert_eq!(
            GatewayBindingService::effective_model(Some("claude-sonnet-4")),
            Some("claude-sonnet-4".to_string())
        );
        assert_eq!(
            GatewayBindingService::effective_model(Some("gpt-5")),
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
        let content = "model = \"gpt-5\"\nexperimental_bearer_token = \"secret\"\nenv_key = \"OPENAI_API_KEY\"\n";
        let redacted = redact_content("toml", content);

        assert!(!redacted.contains("\"secret\""));
        assert!(redacted.contains("model = \"gpt-5\""));
        assert!(redacted.contains("env_key = \"OPENAI_API_KEY\""));
    }

    #[test]
    fn codex_fingerprint_recognizes_embedded_bearer_token() {
        let content = r#"model = "deepseek-v4-flash"
model_provider = "anyrouter"
preferred_auth_method = "apikey"

[model_providers.anyrouter]
name = "DeepSeek"
base_url = "https://api.deepseek.com/v1"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "sk-secret"
"#;

        let fingerprint = codex_fingerprint(
            content,
            "",
            vec![PathBuf::from("/home/u/.codex/anyrouter.config.toml")],
        )
        .unwrap();

        assert!(fingerprint.credential_present);
        assert!(fingerprint.configured);
        assert_eq!(fingerprint.model.as_deref(), Some("deepseek-v4-flash"));
        assert_eq!(fingerprint.endpoint.as_deref(), Some("api.deepseek.com"));
    }

    #[test]
    fn claude_fingerprint_tracks_context_marker_and_credentials() {
        let content = r#"{
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "secret",
                "ANTHROPIC_BASE_URL": "https://relay.example/v1/",
                "ANTHROPIC_DEFAULT_SONNET_MODEL": "claude-sonnet[1m]"
            }
        }"#;
        let fingerprint = claude_fingerprint(content, PathBuf::from("/tmp/settings.json")).unwrap();
        assert!(fingerprint.credential_present);
        assert!(fingerprint.configured);
        assert!(fingerprint.context1m);
        assert_eq!(fingerprint.endpoint.as_deref(), Some("relay.example"));
        assert_eq!(fingerprint.model.as_deref(), Some("claude-sonnet"));
    }

    #[test]
    fn malformed_fingerprint_is_reported_as_parse_failure_by_callers() {
        let result = claude_fingerprint("{not-json", PathBuf::from("/tmp/settings.json"));
        assert!(result.is_err());
    }
}
