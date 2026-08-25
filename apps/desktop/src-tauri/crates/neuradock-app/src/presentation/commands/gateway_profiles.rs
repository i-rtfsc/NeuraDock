use super::gateway::{
    read_agent_fingerprint, AgentProfileDto, CodexConfigFileDto, GatewayClientDto,
};
use super::gateway_fingerprint::claude_fingerprint;
use crate::application::services::{ClaudeConfigService, CodexConfigService};
use crate::presentation::error::CommandError;
use crate::presentation::state::{Repositories, Services};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn abbreviate_home(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(relative) = path.strip_prefix(home) {
            return format!("~/{}", relative.display());
        }
    }
    path.display().to_string()
}

fn expand_home_path(path: &str) -> PathBuf {
    path.strip_prefix("~/")
        .and_then(|relative| dirs::home_dir().map(|home| home.join(relative)))
        .unwrap_or_else(|| PathBuf::from(path))
}

fn profile_credential(client: GatewayClientDto, path: &str) -> Option<String> {
    let content = fs::read_to_string(expand_home_path(path)).ok()?;
    match client {
        GatewayClientDto::Codex => content
            .parse::<toml_edit::DocumentMut>()
            .ok()?
            .get("model_providers")?
            .as_table()?
            .iter()
            .find_map(|(_, provider)| {
                provider
                    .as_table()?
                    .get("experimental_bearer_token")?
                    .as_str()
                    .map(str::to_string)
            }),
        GatewayClientDto::ClaudeCode => {
            let doc = serde_json::from_str::<serde_json::Value>(&content).ok()?;
            doc.pointer("/env/ANTHROPIC_AUTH_TOKEN")
                .or_else(|| doc.pointer("/env/ANTHROPIC_API_KEY"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        }
    }
}

pub(super) async fn match_profile_account(
    client: GatewayClientDto,
    profile: &AgentProfileDto,
    services: &Services,
    repositories: &Repositories,
) -> Result<Option<String>, CommandError> {
    let Some(credential) = profile_credential(client, &profile.path) else {
        return Ok(None);
    };
    let normalized = credential.trim().trim_start_matches("sk-");
    for account in repositories
        .account
        .find_all()
        .await
        .map_err(CommandError::from)?
    {
        let tokens = services
            .token
            .get_cached_tokens(account.id())
            .await
            .map_err(CommandError::from)?;
        if tokens
            .iter()
            .any(|token| token.key().trim().trim_start_matches("sk-") == normalized)
        {
            return Ok(Some(account.name().to_string()));
        }
    }
    Ok(None)
}

pub(super) fn list_codex_config_files(
    has_current: bool,
    current_slug: Option<&str>,
) -> Result<Vec<CodexConfigFileDto>, CommandError> {
    let (official, _) = CodexConfigService::managed_paths().map_err(CommandError::from)?;
    let profiles = official
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| CommandError::infrastructure("Codex config directory is unavailable"))?
        .join("profiles.d");
    let mut targets = vec![("official".to_string(), None, official)];
    if profiles.exists() {
        for entry in fs::read_dir(&profiles).map_err(|e| {
            CommandError::infrastructure(format!("Failed to read {}: {e}", profiles.display()))
        })? {
            let entry = entry.map_err(|e| CommandError::infrastructure(e.to_string()))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(slug) = name.strip_suffix(".config.toml") {
                if !slug.is_empty() && entry.path().is_file() {
                    targets.push(("isolated".into(), Some(slug.into()), entry.path()));
                }
            }
        }
    }
    if let Some(slug) = current_slug {
        if !targets.iter().any(|(_, s, _)| s.as_deref() == Some(slug)) {
            targets.push((
                "isolated".into(),
                Some(slug.into()),
                CodexConfigService::isolated_path(slug).map_err(CommandError::from)?,
            ));
        }
    }
    targets.sort_by(|a, b| a.2.cmp(&b.2));
    targets
        .into_iter()
        .map(|(kind, slug, path)| {
            let fp = read_agent_fingerprint(GatewayClientDto::Codex, slug.as_deref())?;
            Ok(CodexConfigFileDto {
                current: has_current && slug.as_deref() == current_slug,
                kind,
                slug,
                path: abbreviate_home(&path),
                exists: fp.exists,
                parse_ok: fp.parse_ok,
                configured: fp.configured,
                endpoint: fp.endpoint,
                model: fp.model,
                context1m: fp.context1m,
                credential_present: fp.credential_present,
            })
        })
        .collect()
}

pub(super) fn list_claude_profiles() -> Result<Vec<AgentProfileDto>, CommandError> {
    let service = ClaudeConfigService::new();
    let official = ClaudeConfigService::managed_path().map_err(CommandError::from)?;
    let mut paths = vec![("official".into(), None, official)];
    let dir = service
        .isolated_path("profile")
        .map_err(CommandError::from)?
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| CommandError::infrastructure("Claude profiles directory is unavailable"))?;
    if dir.exists() {
        for entry in fs::read_dir(&dir).map_err(|e| {
            CommandError::infrastructure(format!("Failed to read {}: {e}", dir.display()))
        })? {
            let entry = entry.map_err(|e| CommandError::infrastructure(e.to_string()))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(slug) = name
                .strip_prefix("settings-")
                .and_then(|v| v.strip_suffix(".json"))
            {
                if !slug.is_empty() && entry.path().is_file() {
                    paths.push(("isolated".into(), Some(slug.into()), entry.path()));
                }
            }
        }
    }
    paths.sort_by(|a, b| a.2.cmp(&b.2));
    paths
        .into_iter()
        .map(|(kind, slug, path)| {
            let content = if path.exists() {
                fs::read_to_string(&path).unwrap_or_default()
            } else {
                String::new()
            };
            let fp = claude_fingerprint(&content, path.clone()).unwrap_or_default();
            Ok(AgentProfileDto {
                kind,
                slug,
                path: abbreviate_home(&path),
                exists: fp.exists,
                parse_ok: fp.parse_ok,
                configured: fp.configured,
                endpoint: fp.endpoint,
                model: fp.model,
                context1m: fp.context1m,
                credential_present: fp.credential_present,
                managed: false,
                account_label: None,
                gateway_enabled: false,
                method: None,
            })
        })
        .collect()
}
