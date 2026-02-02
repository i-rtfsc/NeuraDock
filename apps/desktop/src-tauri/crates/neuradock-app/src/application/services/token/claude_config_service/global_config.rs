use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;

use super::helpers::{ensure_sk_prefix, get_claude_config_path, MANAGED_ENV_KEYS};
use neuradock_domain::token::ApiToken;

pub(super) fn configure_global_impl(
    token: &ApiToken,
    base_url: &str,
    model: Option<&str>,
) -> Result<String> {
    let api_key = ensure_sk_prefix(token.key());
    configure_global_with_key_impl(&api_key, base_url, model)
}

pub(super) fn configure_global_with_key_impl(
    api_key: &str,
    base_url: &str,
    model: Option<&str>,
) -> Result<String> {
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

    // Get or create env object
    let api_key = ensure_sk_prefix(api_key);

    if let Some(env_value) = config_obj.get_mut("env") {
        // env exists, update it if it's an object
        if let Some(env_obj) = env_value.as_object_mut() {
            env_obj.insert("ANTHROPIC_AUTH_TOKEN".to_string(), json!(api_key));
            env_obj.insert("ANTHROPIC_BASE_URL".to_string(), json!(base_url));
            env_obj.insert(
                "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(),
                json!("1"),
            );
            env_obj.insert("DISABLE_TELEMETRY".to_string(), json!("1"));
            env_obj.insert("API_TIMEOUT_MS".to_string(), json!("3000000"));
            env_obj.insert("CLAUDE_CODE_ATTRIBUTION_HEADER".to_string(), json!("0"));

            if let Some(m) = model {
                env_obj.insert("ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(), json!(m));
                env_obj.insert("ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(), json!(m));
                env_obj.insert("ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(), json!(m));
            }
        } else {
            // env exists but is not an object, replace it
            let mut env_map = serde_json::Map::new();
            env_map.insert("ANTHROPIC_AUTH_TOKEN".to_string(), json!(api_key));
            env_map.insert("ANTHROPIC_BASE_URL".to_string(), json!(base_url));
            env_map.insert(
                "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(),
                json!("1"),
            );
            env_map.insert("DISABLE_TELEMETRY".to_string(), json!("1"));
            env_map.insert("API_TIMEOUT_MS".to_string(), json!("3000000"));
            env_map.insert("CLAUDE_CODE_ATTRIBUTION_HEADER".to_string(), json!("0"));

            if let Some(m) = model {
                env_map.insert("ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(), json!(m));
                env_map.insert("ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(), json!(m));
                env_map.insert("ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(), json!(m));
            }

            *env_value = Value::Object(env_map);
        }
    } else {
        // env doesn't exist, create it
        let mut env_map = serde_json::Map::new();
        env_map.insert("ANTHROPIC_AUTH_TOKEN".to_string(), json!(api_key));
        env_map.insert("ANTHROPIC_BASE_URL".to_string(), json!(base_url));
        env_map.insert(
            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(),
            json!("1"),
        );
        env_map.insert("DISABLE_TELEMETRY".to_string(), json!("1"));
        env_map.insert("API_TIMEOUT_MS".to_string(), json!("3000000"));
        env_map.insert("CLAUDE_CODE_ATTRIBUTION_HEADER".to_string(), json!("0"));

        if let Some(m) = model {
            env_map.insert("ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(), json!(m));
            env_map.insert("ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(), json!(m));
            env_map.insert("ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(), json!(m));
        }

        config_obj.insert("env".to_string(), Value::Object(env_map));
    }

    // Write back config with proper formatting
    let content = serde_json::to_string_pretty(&config)?;
    fs::write(&config_path, content)?;

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
    fs::write(&config_path, content)?;

    log::info!(
        "Successfully cleared Claude Code configuration at: {}",
        config_path.display()
    );

    Ok("Successfully cleared Claude Code global configuration".to_string())
}
