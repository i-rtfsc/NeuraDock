use anyhow::{Context, Result};
use std::path::PathBuf;

use super::helpers::{
    apply_provider_config, ensure_sk_prefix, get_codex_dir, sanitize_provider_slug,
};
use neuradock_domain::token::ApiToken;
use neuradock_infrastructure::fs_safe::{
    atomic_write, backup_file, backup_path_for, remove_backup,
};

/// Canonical managed file: `~/.codex/profiles.d/<slug>.config.toml`.
pub(super) fn isolated_config_path(provider_slug: &str) -> Result<PathBuf> {
    Ok(get_codex_dir()?
        .join("profiles.d")
        .join(format!("{}.config.toml", provider_slug)))
}

/// Codex `-p <slug>` only discovers `$CODEX_HOME/<slug>.config.toml`.
fn isolated_alias_path(provider_slug: &str) -> Result<PathBuf> {
    Ok(get_codex_dir()?.join(format!("{}.config.toml", provider_slug)))
}

fn create_profile_symlink(target: &std::path::Path, alias: &std::path::Path) -> Result<()> {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, alias)?;
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(target, alias)?;
    Ok(())
}

fn prepare_isolated_paths(provider_slug: &str) -> Result<(PathBuf, PathBuf)> {
    let config_path = isolated_config_path(provider_slug)?;
    let alias_path = isolated_alias_path(provider_slug)?;
    let profiles_dir = config_path
        .parent()
        .context("Codex profiles directory missing")?;
    std::fs::create_dir_all(profiles_dir)?;

    if let Ok(metadata) = std::fs::symlink_metadata(&alias_path) {
        if metadata.file_type().is_symlink() {
            let expected = PathBuf::from("profiles.d").join(format!("{provider_slug}.config.toml"));
            let actual = std::fs::read_link(&alias_path)?;
            if actual != expected && actual != config_path {
                anyhow::bail!(
                    "Codex profile alias {} points to an unmanaged target",
                    alias_path.display()
                );
            }
        } else if config_path.exists() {
            anyhow::bail!(
                "Cannot migrate Codex profile because both {} and {} exist",
                alias_path.display(),
                config_path.display()
            );
        } else {
            std::fs::rename(&alias_path, &config_path).with_context(|| {
                format!(
                    "Failed to migrate {} to {}",
                    alias_path.display(),
                    config_path.display()
                )
            })?;
            let old_backup = backup_path_for(&alias_path)?;
            let new_backup = backup_path_for(&config_path)?;
            if old_backup.exists() && !new_backup.exists() {
                std::fs::rename(old_backup, new_backup)?;
            }
        }
    }

    if std::fs::symlink_metadata(&alias_path).is_err() {
        let relative_target =
            PathBuf::from("profiles.d").join(format!("{provider_slug}.config.toml"));
        create_profile_symlink(&relative_target, &alias_path).with_context(|| {
            format!(
                "Failed to create Codex profile alias {}",
                alias_path.display()
            )
        })?;
    }
    Ok((config_path, alias_path))
}

fn remove_managed_alias(provider_slug: &str) -> Result<Option<PathBuf>> {
    let alias = isolated_alias_path(provider_slug)?;
    let Ok(metadata) = std::fs::symlink_metadata(&alias) else {
        return Ok(None);
    };
    if !metadata.file_type().is_symlink() {
        return Ok(None);
    }
    let expected = PathBuf::from("profiles.d").join(format!("{provider_slug}.config.toml"));
    let target = std::fs::read_link(&alias)?;
    if target != expected && target != isolated_config_path(provider_slug)? {
        return Ok(None);
    }
    std::fs::remove_file(&alias)?;
    Ok(Some(alias))
}

/// Legacy key path used by the previous jq-based isolated configuration.
/// New configurations keep the bearer token directly in the TOML file; this
/// path is retained only so stale credential files can be removed and older
/// bindings can still be inspected safely.
pub(super) fn isolated_key_path(provider_slug: &str) -> Result<PathBuf> {
    Ok(get_codex_dir()?
        .join("secrets")
        .join("providers")
        .join(provider_slug)
        .join("api-key.json"))
}

/// Config path for an isolated binding named after `slug_hint`.
pub(super) fn isolated_config_path_for(slug_hint: &str) -> Result<PathBuf> {
    let slug = if slug_hint.trim().is_empty() {
        "openai_compatible".to_string()
    } else {
        sanitize_provider_slug(slug_hint)
    };
    isolated_config_path(&slug)
}

pub(super) fn clear_isolated_impl(slug_hint: &str) -> Result<String> {
    let slug = if slug_hint.trim().is_empty() {
        "openai_compatible".to_string()
    } else {
        sanitize_provider_slug(slug_hint)
    };
    let (config_path, _) = prepare_isolated_paths(&slug)?;
    let legacy_key_path = isolated_key_path(&slug)?;
    let mut removed = Vec::new();

    let backup_path = backup_path_for(&config_path)?;
    if backup_path.exists() {
        let original = std::fs::read_to_string(&backup_path)
            .with_context(|| format!("Failed to read {}", backup_path.display()))?;
        original
            .parse::<toml_edit::DocumentMut>()
            .with_context(|| format!("Backup {} is not valid TOML", backup_path.display()))?;
        atomic_write(&config_path, &original, true)?;
        remove_backup(&config_path)?;
        removed.push(format!("{} (restored)", config_path.display()));
    } else {
        match std::fs::remove_file(&config_path) {
            Ok(()) => removed.push(config_path.display().to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("Failed to remove {}", config_path.display()))
            }
        }
    }

    match std::fs::remove_file(&legacy_key_path) {
        Ok(()) => removed.push(legacy_key_path.display().to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error)
                .with_context(|| format!("Failed to remove {}", legacy_key_path.display()))
        }
    }

    if !config_path.exists() {
        if let Some(alias) = remove_managed_alias(&slug)? {
            removed.push(alias.display().to_string());
        }
    }

    if removed.is_empty() {
        Ok(format!("No isolated Codex config found for {slug}"))
    } else {
        Ok(format!(
            "Removed isolated Codex config:\n  - {}",
            removed.join("\n  - ")
        ))
    }
}

fn build_isolated_config(
    provider_slug: &str,
    provider_name: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
    api_key: &str,
) -> Result<String> {
    let mut doc = toml_edit::DocumentMut::new();
    apply_provider_config(
        &mut doc,
        provider_slug,
        provider_name,
        base_url,
        model,
        context_1m,
    );

    // Keep isolated configs self-contained: Codex reads the API key directly
    // from this provider table and does not need jq or a separate auth.json.
    {
        use toml_edit::value;
        let providers = doc
            .get_mut("model_providers")
            .and_then(|i| i.as_table_mut())
            .context("model_providers table missing")?;
        let entry = providers
            .get_mut(provider_slug)
            .and_then(|i| i.as_table_mut())
            .context("provider table missing")?;

        entry["requires_openai_auth"] = value(false);
        entry["experimental_bearer_token"] = value(ensure_sk_prefix(api_key));
        entry.remove("auth");
    }

    Ok(doc.to_string())
}

pub(super) fn configure_isolated_impl(
    token: &ApiToken,
    provider_id: &str,
    provider_name: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<String> {
    let slug = sanitize_provider_slug(provider_id);
    let display = if provider_name.is_empty() {
        provider_id
    } else {
        provider_name
    };
    configure_isolated(&slug, display, token.key(), base_url, model, context_1m)
}

pub(super) fn configure_isolated_with_key_impl(
    api_key: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<String> {
    // Derive a slug from the base_url host, fallback to "openai_compatible".
    let slug = url::Url::parse(base_url)
        .ok()
        .and_then(|u| u.host_str().map(|h| sanitize_provider_slug(h)))
        .unwrap_or_else(|| "openai_compatible".to_string());
    configure_isolated(&slug, &slug, api_key, base_url, model, context_1m)
}

/// Write an isolated config under an explicit provider slug/name. Used by the
/// gateway path, where the file is named after the real relay provider but its
/// `base_url` points at the local gateway (`gateway_base_url`) and the key is
/// the gateway's process-local credential. Never snapshots or touches the
/// official config.toml/auth.json.
pub(super) fn configure_isolated_gateway_impl(
    slug_hint: &str,
    display: &str,
    gateway_base_url: &str,
    local_credential: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<String> {
    let slug = if slug_hint.trim().is_empty() {
        "openai_compatible".to_string()
    } else {
        sanitize_provider_slug(slug_hint)
    };
    let display = if display.trim().is_empty() {
        slug.as_str()
    } else {
        display
    };
    configure_isolated(
        &slug,
        display,
        local_credential,
        gateway_base_url,
        model,
        context_1m,
    )
}

fn configure_isolated(
    provider_slug: &str,
    provider_name: &str,
    api_key: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<String> {
    let (config_path, alias_path) = prepare_isolated_paths(provider_slug)?;
    let legacy_key_path = isolated_key_path(provider_slug)?;

    let config_content = build_isolated_config(
        provider_slug,
        provider_name,
        base_url,
        model,
        context_1m,
        api_key,
    )?;

    // The file may already be a user-maintained relay config. Preserve it so
    // unbinding can restore the exact previous contents instead of deleting it.
    if !backup_path_for(&config_path)?.exists() {
        backup_file(&config_path)?;
    }

    // The config now contains a credential, so write it with private
    // permissions. Remove the old jq key file after the replacement succeeds.
    atomic_write(&config_path, &config_content, true)?;
    if legacy_key_path.exists() {
        std::fs::remove_file(&legacy_key_path).with_context(|| {
            format!(
                "Failed to remove legacy Codex key file {}",
                legacy_key_path.display()
            )
        })?;
    }

    log::info!("Isolated Codex config written: {}", config_path.display());
    Ok(format!(
        "Isolated config written:\n  - {}\nProfile command: codex -p {}\nAlias: {}",
        config_path.display(),
        provider_slug,
        alias_path.display()
    ))
}

pub(super) fn preview_isolated_impl(
    token: &ApiToken,
    provider_id: &str,
    provider_name: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<Vec<(PathBuf, &'static str, String, String)>> {
    let slug = sanitize_provider_slug(provider_id);
    let display = if provider_name.is_empty() {
        provider_id
    } else {
        provider_name
    };
    preview_isolated(&slug, display, token.key(), base_url, model, context_1m)
}

pub(super) fn preview_isolated_with_key_impl(
    api_key: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<Vec<(PathBuf, &'static str, String, String)>> {
    let slug = url::Url::parse(base_url)
        .ok()
        .and_then(|u| u.host_str().map(|h| sanitize_provider_slug(h)))
        .unwrap_or_else(|| "openai_compatible".to_string());
    preview_isolated(&slug, &slug, api_key, base_url, model, context_1m)
}

/// Preview the isolated files a gateway binding would write: named after the
/// real relay (`slug_hint`/`display`) but pointing at the local gateway.
/// Mirrors `configure_isolated_gateway_impl` so preview and write cannot drift.
pub(super) fn preview_isolated_gateway_impl(
    slug_hint: &str,
    display: &str,
    gateway_base_url: &str,
    local_credential: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<Vec<(PathBuf, &'static str, String, String)>> {
    let slug = if slug_hint.trim().is_empty() {
        "openai_compatible".to_string()
    } else {
        sanitize_provider_slug(slug_hint)
    };
    let display = if display.trim().is_empty() {
        slug.as_str()
    } else {
        display
    };
    preview_isolated(
        &slug,
        display,
        local_credential,
        gateway_base_url,
        model,
        context_1m,
    )
}

fn preview_isolated(
    provider_slug: &str,
    provider_name: &str,
    api_key: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<Vec<(PathBuf, &'static str, String, String)>> {
    let config_path = isolated_config_path(provider_slug)?;

    let alias_path = isolated_alias_path(provider_slug)?;
    let current_path = if config_path.exists() {
        &config_path
    } else {
        &alias_path
    };
    let current_config = if current_path.exists() {
        std::fs::read_to_string(current_path)?
    } else {
        String::new()
    };
    let next_config = build_isolated_config(
        provider_slug,
        provider_name,
        base_url,
        model,
        context_1m,
        api_key,
    )?;

    Ok(vec![(config_path, "toml", current_config, next_config)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_config_embeds_bearer_token_without_jq() {
        let toml = build_isolated_config(
            "agentrouter",
            "AgentRouter",
            "https://relay.example",
            Some("gpt-5.6-sol"),
            false,
            "secret-key",
        )
        .unwrap();

        let doc: toml_edit::DocumentMut = toml.parse().unwrap();
        // Top-level uses the API key authentication method.
        assert_eq!(doc["model"].as_str(), Some("gpt-5.6-sol"));
        assert_eq!(doc["model_provider"].as_str(), Some("agentrouter"));
        assert_eq!(doc["preferred_auth_method"].as_str(), Some("apikey"));
        assert!(doc.get("requires_openai_auth").is_none());

        // Provider table carries the credential directly and has no auth command.
        let provider = doc["model_providers"]["agentrouter"].as_table().unwrap();
        assert_eq!(
            provider["base_url"].as_str(),
            Some("https://relay.example/v1")
        );
        assert_eq!(provider["requires_openai_auth"].as_bool(), Some(false));
        assert_eq!(
            provider["experimental_bearer_token"].as_str(),
            Some("sk-secret-key")
        );
        assert!(provider.get("auth").is_none());
    }

    #[test]
    fn isolated_config_uses_profiles_directory() {
        let path = isolated_config_path("agentrouter").unwrap();
        assert!(path.ends_with(".codex/profiles.d/agentrouter.config.toml"));
    }
}
