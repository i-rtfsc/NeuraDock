use super::gateway::{
    read_agent_fingerprint, AgentProfileDto, CodexConfigFileDto, GatewayClientDto,
};
use super::gateway_fingerprint::claude_fingerprint;
use crate::application::services::{ClaudeConfigService, CodexConfigService};
use crate::presentation::error::CommandError;
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
