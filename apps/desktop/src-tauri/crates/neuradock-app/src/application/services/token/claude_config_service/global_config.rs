use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::sync::{LazyLock, Mutex};

use super::helpers::{ensure_sk_prefix, get_claude_config_path, with_1m_suffix, MANAGED_ENV_KEYS};
use neuradock_domain::token::ApiToken;
use neuradock_infrastructure::fs_safe::{atomic_write, backup_file};

/// Serializes read-modify-write access to ~/.claude/settings.json so two
/// concurrent configure/clear calls cannot interleave.
static CLAUDE_CONFIG_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub(super) fn configure_global_impl(
    token: &ApiToken,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<String> {
    let api_key = ensure_sk_prefix(token.key());
    configure_global_with_key_impl(&api_key, base_url, model, context_1m)
}

pub(super) fn configure_global_with_key_impl(
    api_key: &str,
    base_url: &str,
    model: Option<&str>,
    context_1m: bool,
) -> Result<String> {
    let _guard = CLAUDE_CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // The 1M option rides on the model id, so it only applies when a model
    // was selected; the command layer rejects 1M without a model.
    let model_with_context = model.map(|m| {
        if context_1m {
            with_1m_suffix(m)
        } else {
            m.to_string()
        }
    });
    let model = model_with_context.as_deref();

    let config_path = get_claude_config_path()?;

    // Ensure directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Read existing config or create empty object
    let mut config: Value = if config_path.exists() {
        let content =
            fs::read_to_string(&config_path).context("Failed to read existing settings.json")?;
        serde_json::from_str(&content).context("Failed to parse existing settings.json")?
    } else {
        json!({})
    };

    // Ensure config is an object
    let config_obj = config
        .as_object_mut()
        .context("settings.json must be a JSON object")?;

    // Build the managed env keys once, then merge them into the existing
    // env object so user-defined variables survive untouched.
    let api_key = ensure_sk_prefix(api_key);
    let mut env_updates = serde_json::Map::new();
    env_updates.insert("ANTHROPIC_AUTH_TOKEN".to_string(), json!(api_key));
    env_updates.insert("ANTHROPIC_BASE_URL".to_string(), json!(base_url));
    env_updates.insert(
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(),
        json!("1"),
    );
    env_updates.insert("DISABLE_TELEMETRY".to_string(), json!("1"));
    env_updates.insert("API_TIMEOUT_MS".to_string(), json!("3000000"));
    env_updates.insert("CLAUDE_CODE_ATTRIBUTION_HEADER".to_string(), json!("0"));

    if let Some(m) = model {
        env_updates.insert("ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(), json!(m));
        env_updates.insert("ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(), json!(m));
        env_updates.insert("ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(), json!(m));
    }

    match config_obj.get_mut("env") {
        Some(Value::Object(env_obj)) => {
            env_obj.extend(env_updates);
        }
        Some(other) => {
            // env exists but is not an object, replace it
            *other = Value::Object(env_updates);
        }
        None => {
            config_obj.insert("env".to_string(), Value::Object(env_updates));
        }
    }

    // Write back config with proper formatting
    let content = serde_json::to_string_pretty(&config)?;
    backup_file(&config_path)?;
    atomic_write(&config_path, &content, true)?;

    log::info!(
        "Successfully configured Claude Code at: {}",
        config_path.display()
    );

    Ok(format!(
        "Successfully configured Claude Code globally at: {}",
        config_path.display()
    ))
}

pub(super) fn clear_global_impl() -> Result<String> {
    let _guard = CLAUDE_CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let config_path = get_claude_config_path()?;

    if !config_path.exists() {
        return Ok("No Claude Code configuration file found".to_string());
    }

    // Read existing config
    let content =
        fs::read_to_string(&config_path).context("Failed to read existing settings.json")?;
    let mut config: Value =
        serde_json::from_str(&content).context("Failed to parse existing settings.json")?;

    // Ensure config is an object
    let config_obj = config
        .as_object_mut()
        .context("settings.json must be a JSON object")?;

    // Remove only our managed keys from env
    if let Some(env_value) = config_obj.get_mut("env") {
        if let Some(env_obj) = env_value.as_object_mut() {
            for key in MANAGED_ENV_KEYS {
                env_obj.remove(*key);
            }

            // If env is now empty, remove it entirely
            if env_obj.is_empty() {
                config_obj.remove("env");
            }
        }
    }

    // Write back config
    let content = serde_json::to_string_pretty(&config)?;
    backup_file(&config_path)?;
    atomic_write(&config_path, &content, true)?;

    log::info!(
        "Successfully cleared Claude Code configuration at: {}",
        config_path.display()
    );

    Ok("Successfully cleared Claude Code global configuration".to_string())
}
