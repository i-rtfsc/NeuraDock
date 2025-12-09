use anyhow::{Context, Result};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

use neuradock_domain::token::ApiToken;

pub struct CodexConfigService;

impl CodexConfigService {
    pub fn new() -> Self {
        Self
    }

    fn get_codex_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Cannot find home directory")?;
        Ok(home.join(".codex"))
    }

    fn get_codex_config_path() -> Result<PathBuf> {
        Ok(Self::get_codex_dir()?.join("config.toml"))
    }

    fn get_codex_auth_path() -> Result<PathBuf> {
        Ok(Self::get_codex_dir()?.join("auth.json"))
    }

    /// Ensure API key has sk- prefix
    fn ensure_sk_prefix(key: &str) -> String {
        if key.starts_with("sk-") {
            key.to_string()
        } else {
            format!("sk-{}", key)
        }
    }

    /// Generate config.toml content for AnyRouter
    fn generate_anyrouter_config(base_url: &str, model: Option<&str>) -> String {
        let model_name = model.unwrap_or("gpt-5-codex");
        format!(
            r#"model = "{}"
model_provider = "anyrouter"
preferred_auth_method = "apikey"


[model_providers.anyrouter]
name = "Any Router"
base_url = "{}/v1"
wire_api = "responses"
"#,
            model_name, base_url
        )
    }

    /// Generate config.toml content for AgentRouter
    fn generate_agentrouter_config(base_url: &str, model: Option<&str>) -> String {
        let model_name = model.unwrap_or("gpt-5");
        format!(
            r#"model = "{}"
model_provider = "openai-chat-completions"
preferred_auth_method = "apikey"


[model_providers.openai-chat-completions]
name = "OpenAI using Chat Completions"
base_url = "{}/v1"
#env_key = "AGENT_ROUTER_TOKEN"
wire_api = "chat"
query_params = {{}}
stream_idle_timeout_ms = 300000
"#,
            model_name, base_url
        )
    }

    /// Configure Codex globally by writing to ~/.codex/config.toml and ~/.codex/auth.json
    pub fn configure_global(
        &self,
        token: &ApiToken,
        provider_id: &str,
        base_url: &str,
        model: Option<&str>,
    ) -> Result<String> {
        let codex_dir = Self::get_codex_dir()?;
        let config_path = Self::get_codex_config_path()?;
        let auth_path = Self::get_codex_auth_path()?;

        // Ensure directory exists
        fs::create_dir_all(&codex_dir)?;

        // Generate config.toml based on provider
        let config_content = match provider_id {
            "anyrouter" => Self::generate_anyrouter_config(base_url, model),
            "agentrouter" => Self::generate_agentrouter_config(base_url, model),
            _ => return Err(anyhow::anyhow!("Unsupported provider: {}", provider_id)),
        };

        // Write config.toml
        fs::write(&config_path, &config_content)?;
        log::info!("Codex config.toml written to: {}", config_path.display());

        // Create auth.json with API key (ensure sk- prefix)
        let api_key = Self::ensure_sk_prefix(token.key());
        let auth_content = json!({
            "OPENAI_API_KEY": api_key
        });

        let auth_json = serde_json::to_string_pretty(&auth_content)?;
        fs::write(&auth_path, auth_json)?;
        log::info!("Codex auth.json written to: {}", auth_path.display());

        Ok(format!(
            "Successfully configured Codex globally:\n  - config.toml: {}\n  - auth.json: {}",
            config_path.display(),
            auth_path.display()
        ))
    }

    /// Clear Codex global configuration
    /// Removes both config.toml and auth.json files
    pub fn clear_global(&self) -> Result<String> {
        let config_path = Self::get_codex_config_path()?;
        let auth_path = Self::get_codex_auth_path()?;
        let mut removed = vec![];

        if config_path.exists() {
            fs::remove_file(&config_path)?;
            removed.push(format!("config.toml: {}", config_path.display()));
        }

        if auth_path.exists() {
            fs::remove_file(&auth_path)?;
            removed.push(format!("auth.json: {}", auth_path.display()));
        }

        if removed.is_empty() {
            return Ok("No Codex configuration files found".to_string());
        }

        log::info!("Successfully cleared Codex configuration");

        Ok(format!(
            "Successfully cleared Codex configuration:\n  - {}",
            removed.join("\n  - ")
        ))
    }

    /// Generate temporary export commands for current shell session
    /// Note: This feature is temporarily unavailable
    #[allow(dead_code)]
    pub fn generate_temp_commands(
        &self,
        _token: &ApiToken,
        _provider_id: &str,
        _base_url: &str,
        _model: Option<&str>,
    ) -> Result<String> {
        Err(anyhow::anyhow!(
            "Temporary configuration is currently unavailable. Please use global configuration instead."
        ))
    }
}

impl Default for CodexConfigService {
    fn default() -> Self {
        Self::new()
    }
}
