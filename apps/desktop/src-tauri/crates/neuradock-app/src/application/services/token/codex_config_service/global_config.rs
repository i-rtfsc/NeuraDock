use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

use super::helpers::{
    apply_provider_config, ensure_sk_prefix, get_codex_auth_path, get_codex_config_path,
    sanitize_provider_slug, GENERIC_PROVIDER_NAME, GENERIC_PROVIDER_SLUG, MANAGED_TOP_LEVEL_KEYS,
};
use neuradock_domain::token::ApiToken;
use neuradock_infrastructure::fs_safe::{atomic_write, backup_file, remove_backup};

/// Serializes all read-modify-write access to the Codex config files so two
/// concurrent "configure"/"clear" calls cannot interleave and clobber each
/// other's changes.
static CODEX_CONFIG_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub(super) fn configure_global_impl(
    token: &ApiToken,
    provider_id: &str,
    provider_name: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<String> {
    let provider_slug = sanitize_provider_slug(provider_id);
    let display_name = if provider_name.is_empty() {
        provider_id
    } else {
        provider_name
    };

    configure(
        &provider_slug,
        display_name,
        token.key(),
        base_url,
        model,
        context_1m,
    )
}

pub(super) fn configure_global_with_key_impl(
    api_key: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<String> {
    configure(
        GENERIC_PROVIDER_SLUG,
        GENERIC_PROVIDER_NAME,
        api_key,
        base_url,
        model,
        context_1m,
    )
}

/// Surgically update ~/.codex/config.toml and ~/.codex/auth.json: only the
/// provider fields we own change, every other key the user wrote survives.
fn configure(
    provider_slug: &str,
    provider_name: &str,
    api_key: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<String> {
    let _guard = CODEX_CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let config_path = get_codex_config_path()?;
    let auth_path = get_codex_auth_path()?;

    let (doc, auth) = build_documents(
        load_toml_document(&config_path)?,
        load_json_object(&auth_path)?,
        provider_slug,
        provider_name,
        api_key,
        base_url,
        model,
        context_1m,
    )?;

    // config.toml: read-modify-write, preserving unrelated keys.
    backup_file(&config_path)?;
    atomic_write(&config_path, &doc.to_string(), false)?;
    log::info!("Codex config.toml updated at: {}", config_path.display());

    // auth.json: upsert OPENAI_API_KEY, keep any other fields.
    backup_file(&auth_path)?;
    atomic_write(&auth_path, &serde_json::to_string_pretty(&auth)?, true)?;
    log::info!("Codex auth.json updated at: {}", auth_path.display());

    Ok(format!(
        "Successfully configured Codex globally:\n  - config.toml: {}\n  - auth.json: {}",
        config_path.display(),
        auth_path.display()
    ))
}

pub(super) fn preview_global_impl(
    token: &ApiToken,
    provider_id: &str,
    provider_name: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<Vec<(std::path::PathBuf, &'static str, String, String)>> {
    let provider_slug = sanitize_provider_slug(provider_id);
    let display_name = if provider_name.is_empty() {
        provider_id
    } else {
        provider_name
    };
    preview(
        &provider_slug,
        display_name,
        token.key(),
        base_url,
        model,
        context_1m,
    )
}

pub(super) fn preview_global_with_key_impl(
    api_key: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<Vec<(std::path::PathBuf, &'static str, String, String)>> {
    preview(
        GENERIC_PROVIDER_SLUG,
        GENERIC_PROVIDER_NAME,
        api_key,
        base_url,
        model,
        context_1m,
    )
}

fn preview(
    provider_slug: &str,
    provider_name: &str,
    api_key: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<Vec<(std::path::PathBuf, &'static str, String, String)>> {
    let _guard = CODEX_CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let config_path = get_codex_config_path()?;
    let auth_path = get_codex_auth_path()?;
    let current_config = read_if_exists(&config_path)?;
    let current_auth = read_if_exists(&auth_path)?;
    let (next_config, next_auth) = build_documents(
        parse_toml_content(&config_path, &current_config)?,
        parse_json_content(&auth_path, &current_auth)?,
        provider_slug,
        provider_name,
        api_key,
        base_url,
        model,
        context_1m,
    )?;

    Ok(vec![
        (config_path, "toml", current_config, next_config.to_string()),
        (
            auth_path,
            "json",
            current_auth,
            serde_json::to_string_pretty(&next_auth)?,
        ),
    ])
}

fn build_documents(
    mut doc: toml_edit::DocumentMut,
    mut auth: Value,
    provider_slug: &str,
    provider_name: &str,
    api_key: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<(toml_edit::DocumentMut, Value)> {
    apply_provider_config(
        &mut doc,
        provider_slug,
        provider_name,
        base_url,
        model,
        context_1m,
    );
    let auth_obj = auth
        .as_object_mut()
        .context("auth.json must be a JSON object")?;
    auth_obj.insert(
        "OPENAI_API_KEY".to_string(),
        json!(ensure_sk_prefix(api_key)),
    );
    Ok((doc, auth))
}

fn read_if_exists(path: &Path) -> Result<String> {
    if !path.exists() {
        return Ok(String::new());
    }
    fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))
}

fn parse_toml_content(path: &Path, content: &str) -> Result<toml_edit::DocumentMut> {
    if content.trim().is_empty() {
        return Ok(toml_edit::DocumentMut::new());
    }
    content.parse::<toml_edit::DocumentMut>().with_context(|| {
        format!(
            "{} is not valid TOML; fix or remove it manually, then retry",
            path.display()
        )
    })
}

fn parse_json_content(path: &Path, content: &str) -> Result<Value> {
    if content.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(content).with_context(|| {
        format!(
            "{} is not valid JSON; fix or remove it manually, then retry",
            path.display()
        )
    })
}

pub(super) fn clear_global_impl() -> Result<String> {
    let _guard = CODEX_CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let config_path = get_codex_config_path()?;
    let auth_path = get_codex_auth_path()?;
    let mut cleared = vec![];

    // config.toml: remove only the keys this feature manages.
    if config_path.exists() {
        let mut doc = load_toml_document(&config_path)?;

        // Drop the provider table the active model_provider points at.
        let active_slug = doc
            .get("model_provider")
            .and_then(|item| item.as_str())
            .map(|slug| slug.to_string());
        if let Some(slug) = active_slug {
            if let Some(providers) = doc
                .get_mut("model_providers")
                .and_then(|i| i.as_table_mut())
            {
                providers.remove(&slug);
                if providers.is_empty() {
                    doc.remove("model_providers");
                }
            }
        }
        for key in MANAGED_TOP_LEVEL_KEYS {
            doc.remove(key);
        }

        backup_file(&config_path)?;
        atomic_write(&config_path, &doc.to_string(), false)?;
        cleared.push(format!("config.toml: {}", config_path.display()));
    }

    // auth.json: remove only OPENAI_API_KEY; other auth state stays.
    if auth_path.exists() {
        let mut auth = load_json_object(&auth_path)?;
        if let Some(auth_obj) = auth.as_object_mut() {
            auth_obj.remove("OPENAI_API_KEY");
            backup_file(&auth_path)?;
            if auth_obj.is_empty() {
                fs::remove_file(&auth_path)?;
            } else {
                atomic_write(&auth_path, &serde_json::to_string_pretty(&auth)?, true)?;
            }
            cleared.push(format!("auth.json: {}", auth_path.display()));
        }
    }

    if cleared.is_empty() {
        return Ok("No Codex configuration files found".to_string());
    }

    // Both backups above were taken while the files still held the credential
    // this operation exists to remove (OPENAI_API_KEY in auth.json, the relay
    // base URL and key material in config.toml). Keeping them would leave a
    // working key on disk after the user explicitly cleared it.
    for path in [&config_path, &auth_path] {
        if let Err(e) = remove_backup(path) {
            log::warn!(
                "Cleared Codex config but failed to remove credential backup for {}: {}",
                path.display(),
                e
            );
        }
    }

    log::info!("Successfully cleared Codex configuration");

    Ok(format!(
        "Successfully cleared Codex configuration:\n  - {}",
        cleared.join("\n  - ")
    ))
}

/// Parse an existing config.toml, or start from an empty document. A file
/// that exists but fails to parse aborts the operation instead of being
/// silently overwritten.
fn load_toml_document(path: &Path) -> Result<toml_edit::DocumentMut> {
    parse_toml_content(path, &read_if_exists(path)?)
}

/// Parse an existing JSON file, or start from an empty object. A corrupt
/// file aborts instead of being clobbered.
fn load_json_object(path: &Path) -> Result<Value> {
    parse_json_content(path, &read_if_exists(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_builder_preserves_unmanaged_auth_fields() {
        let mut doc = toml_edit::DocumentMut::new();
        doc["approval_policy"] = toml_edit::value("on-request");
        let auth = json!({ "tokens": { "access_token": "kept" } });

        let (doc, auth) = build_documents(
            doc,
            auth,
            "relay",
            "Relay",
            "test-key",
            "https://relay.example/v1",
            Some("gpt-5"),
            false,
        )
        .unwrap();

        assert_eq!(doc["approval_policy"].as_str(), Some("on-request"));
        assert_eq!(auth["tokens"]["access_token"], "kept");
        assert_eq!(auth["OPENAI_API_KEY"], "sk-test-key");
    }
}
