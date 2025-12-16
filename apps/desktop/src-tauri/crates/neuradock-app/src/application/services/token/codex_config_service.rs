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

    fn sanitize_provider_slug(provider_id: &str) -> String {
        provider_id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect()
    }

    fn ensure_v1_base_url(base_url: &str) -> String {
        let trimmed = base_url.trim_end_matches('/');
        if trimmed.ends_with("/v1") {
            trimmed.to_string()
        } else {
            format!("{}/v1", trimmed)
        }
    }

    fn generate_provider_config(
        provider_slug: &str,
        provider_name: &str,
        base_url: &str,
        model: Option<&str>,
    ) -> String {
        let model_name = model.unwrap_or("gpt-5");
        let base_url_v1 = Self::ensure_v1_base_url(base_url);

        format!(
            r#"model = "{}"
model_provider = "{}"
preferred_auth_method = "apikey"


[model_providers.{}]
name = "{}"
base_url = "{}"
wire_api = "responses"
"#,
            model_name, provider_slug, provider_slug, provider_name, base_url_v1
        )
    }

    /// Generate config.toml content for AgentRouter
    /// Generate config.toml content for generic OpenAI-compatible provider (for independent keys)
    fn generate_generic_config(base_url: &str, model: Option<&str>) -> String {
        let model_name = model.unwrap_or("gpt-4o");
        let base_url_v1 = Self::ensure_v1_base_url(base_url);

        format!(
            r#"model = "{}"
model_provider = "openai_compatible"
preferred_auth_method = "apikey"


[model_providers.openai_compatible]
name = "OpenAI Compatible API"
base_url = "{}"
wire_api = "responses"
"#,
            model_name, base_url_v1
        )
    }

    /// Configure Codex globally by writing to ~/.codex/config.toml and ~/.codex/auth.json
    pub fn configure_global(
        &self,
        token: &ApiToken,
        provider_id: &str,
        provider_name: &str,
        base_url: &str,
        model: Option<&str>,
    ) -> Result<String> {
        let codex_dir = Self::get_codex_dir()?;
        let config_path = Self::get_codex_config_path()?;
        let auth_path = Self::get_codex_auth_path()?;

        // Ensure directory exists
        fs::create_dir_all(&codex_dir)?;

        // Generate config.toml based on provider metadata
        let provider_slug = Self::sanitize_provider_slug(provider_id);
        let display_name = if provider_name.is_empty() {
            provider_id
        } else {
            provider_name
        };
        let config_content =
            Self::generate_provider_config(&provider_slug, display_name, base_url, model);

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
        _provider_name: &str,
        _base_url: &str,
        _model: Option<&str>,
    ) -> Result<String> {
        Err(anyhow::anyhow!(
            "Temporary configuration is currently unavailable. Please use global configuration instead."
        ))
    }

    /// Configure Codex globally with API key string (for independent keys)
    pub fn configure_global_with_key(
        &self,
        api_key: &str,
        base_url: &str,
        model: Option<&str>,
    ) -> Result<String> {
        let codex_dir = Self::get_codex_dir()?;
        let config_path = Self::get_codex_config_path()?;
        let auth_path = Self::get_codex_auth_path()?;

        // Ensure directory exists
        fs::create_dir_all(&codex_dir)?;

        // Generate generic OpenAI-compatible config
        let config_content = Self::generate_generic_config(base_url, model);

        // Write config.toml
        fs::write(&config_path, &config_content)?;
        log::info!("Codex config.toml written to: {}", config_path.display());

        // Create auth.json with API key (ensure sk- prefix)
        let api_key = Self::ensure_sk_prefix(api_key);
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

    /// Generate temporary export commands with API key string (for independent keys)
    #[allow(dead_code)]
    pub fn generate_temp_commands_with_key(
        &self,
        _api_key: &str,
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
