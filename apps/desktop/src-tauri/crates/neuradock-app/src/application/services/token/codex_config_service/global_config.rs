use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

use super::helpers::{
    apply_provider_config, ensure_sk_prefix, get_codex_auth_path, get_codex_config_path,
    sanitize_provider_slug, GENERIC_PROVIDER_NAME, GENERIC_PROVIDER_SLUG, MANAGED_TOP_LEVEL_KEYS,
};
use neuradock_domain::token::ApiToken;
use neuradock_infrastructure::fs_safe::{atomic_write, backup_file};

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
    let config_path = get_codex_config_path()?;
    let auth_path = get_codex_auth_path()?;

    // config.toml: read-modify-write, preserving unrelated keys.
    let mut doc = load_toml_document(&config_path)?;
    apply_provider_config(
        &mut doc,
        provider_slug,
        provider_name,
        base_url,
        model,
        context_1m,
    );
    backup_file(&config_path)?;
    atomic_write(&config_path, &doc.to_string(), false)?;
    log::info!("Codex config.toml updated at: {}", config_path.display());

    // auth.json: upsert OPENAI_API_KEY, keep any other fields.
    let mut auth = load_json_object(&auth_path)?;
    let auth_obj = auth
        .as_object_mut()
        .context("auth.json must be a JSON object")?;
    auth_obj.insert(
        "OPENAI_API_KEY".to_string(),
        json!(ensure_sk_prefix(api_key)),
    );
    backup_file(&auth_path)?;
    atomic_write(&auth_path, &serde_json::to_string_pretty(&auth)?, true)?;
    log::info!("Codex auth.json updated at: {}", auth_path.display());

    Ok(format!(
        "Successfully configured Codex globally:\n  - config.toml: {}\n  - auth.json: {}",
        config_path.display(),
        auth_path.display()
    ))
}

pub(super) fn clear_global_impl() -> Result<String> {
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
    if !path.exists() {
        return Ok(toml_edit::DocumentMut::new());
    }
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    content.parse::<toml_edit::DocumentMut>().with_context(|| {
        format!(
            "{} is not valid TOML; fix or remove it manually, then retry",
            path.display()
        )
    })
}

/// Parse an existing JSON file, or start from an empty object. A corrupt
/// file aborts instead of being clobbered.
fn load_json_object(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| {
        format!(
            "{} is not valid JSON; fix or remove it manually, then retry",
            path.display()
        )
    })
}
