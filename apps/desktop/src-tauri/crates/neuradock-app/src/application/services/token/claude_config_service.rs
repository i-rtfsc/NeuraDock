use anyhow::{Context, Result};
use serde_json::{json, Map, Value};
use std::path::PathBuf;
use std::fs;

use neuradock_domain::token::ApiToken;

pub struct ClaudeConfigService;

// Keys that we manage in the env section
const MANAGED_ENV_KEYS: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_BASE_URL",
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
    "DISABLE_TELEMETRY",
    "API_TIMEOUT_MS",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
];

impl ClaudeConfigService {
    pub fn new() -> Self {
        Self
    }

    fn get_claude_config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Cannot find home directory")?;
        Ok(home.join(".claude").join("settings.json"))
    }

    /// Ensure API key has sk- prefix
    fn ensure_sk_prefix(key: &str) -> String {
        if key.starts_with("sk-") {
            key.to_string()
        } else {
            format!("sk-{}", key)
        }
    }

    /// Configure Claude Code globally by writing to ~/.claude/settings.json
    /// This properly merges with existing configuration
    pub fn configure_global(
        &self,
        token: &ApiToken,
        base_url: &str,
        model: Option<&str>,
    ) -> Result<String> {
        let config_path = Self::get_claude_config_path()?;

        // Ensure directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Read existing config or create empty object
        let mut config: Value = if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .context("Failed to read existing settings.json")?;
            serde_json::from_str(&content)
                .context("Failed to parse existing settings.json")?
        } else {
            json!({})
        };

        // Ensure config is an object
        let config_obj = config.as_object_mut()
            .context("settings.json must be a JSON object")?;

        // Get or create env object
        let api_key = Self::ensure_sk_prefix(token.key());

        if let Some(env_value) = config_obj.get_mut("env") {
            // env exists, update it if it's an object
            if let Some(env_obj) = env_value.as_object_mut() {
                env_obj.insert("ANTHROPIC_API_KEY".to_string(), json!(api_key));
                env_obj.insert("ANTHROPIC_BASE_URL".to_string(), json!(base_url));
                env_obj.insert("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(), json!("1"));
                env_obj.insert("DISABLE_TELEMETRY".to_string(), json!("1"));
                env_obj.insert("API_TIMEOUT_MS".to_string(), json!("3000000"));
                
                if let Some(m) = model {
                    env_obj.insert("ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(), json!(m));
                    env_obj.insert("ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(), json!(m));
                    env_obj.insert("ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(), json!(m));
                }
            } else {
                // env exists but is not an object, replace it
                let mut env_map = serde_json::Map::new();
                env_map.insert("ANTHROPIC_API_KEY".to_string(), json!(api_key));
                env_map.insert("ANTHROPIC_BASE_URL".to_string(), json!(base_url));
                env_map.insert("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(), json!("1"));
                env_map.insert("DISABLE_TELEMETRY".to_string(), json!("1"));
                env_map.insert("API_TIMEOUT_MS".to_string(), json!("3000000"));
                
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
            env_map.insert("ANTHROPIC_API_KEY".to_string(), json!(api_key));
            env_map.insert("ANTHROPIC_BASE_URL".to_string(), json!(base_url));
            env_map.insert("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(), json!("1"));
            env_map.insert("DISABLE_TELEMETRY".to_string(), json!("1"));
            env_map.insert("API_TIMEOUT_MS".to_string(), json!("3000000"));
            
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

        log::info!("Successfully configured Claude Code at: {}", config_path.display());

        Ok(format!(
            "Successfully configured Claude Code globally at: {}",
            config_path.display()
        ))
    }

    /// Clear Claude Code global configuration
    /// Only removes the env keys that we manage, preserves other settings
    pub fn clear_global(&self) -> Result<String> {
        let config_path = Self::get_claude_config_path()?;

        if !config_path.exists() {
            return Ok("No Claude Code configuration file found".to_string());
        }

        // Read existing config
        let content = fs::read_to_string(&config_path)
            .context("Failed to read existing settings.json")?;
        let mut config: Value = serde_json::from_str(&content)
            .context("Failed to parse existing settings.json")?;

        // Ensure config is an object
        let config_obj = config.as_object_mut()
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

        log::info!("Successfully cleared Claude Code configuration at: {}", config_path.display());

        Ok("Successfully cleared Claude Code global configuration".to_string())
    }

    /// Generate temporary export commands for current shell session
    pub fn generate_temp_commands(
        &self,
        token: &ApiToken,
        base_url: &str,
        model: Option<&str>,
    ) -> Result<String> {
        let api_key = Self::ensure_sk_prefix(token.key());
        let mut commands = vec![
            format!("export ANTHROPIC_API_KEY=\"{}\"", api_key),
            format!("export ANTHROPIC_BASE_URL=\"{}\"", base_url),
            "export CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=\"1\"".to_string(),
            "export DISABLE_TELEMETRY=\"1\"".to_string(),
            "export API_TIMEOUT_MS=\"3000000\"".to_string(),
        ];
        
        if let Some(m) = model {
            commands.push(format!("export ANTHROPIC_DEFAULT_HAIKU_MODEL=\"{}\"", m));
            commands.push(format!("export ANTHROPIC_DEFAULT_SONNET_MODEL=\"{}\"", m));
            commands.push(format!("export ANTHROPIC_DEFAULT_OPUS_MODEL=\"{}\"", m));
        }
        
        commands.push("# Run the above commands in your terminal, then start Claude Code".to_string());

        Ok(commands.join("\n"))
    }
}

impl Default for ClaudeConfigService {
    fn default() -> Self {
        Self::new()
    }
}
