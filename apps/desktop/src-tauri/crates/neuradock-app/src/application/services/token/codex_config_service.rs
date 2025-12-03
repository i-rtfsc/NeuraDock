use anyhow::{Context, Result};
use serde_json::json;
use std::path::PathBuf;
use std::fs;

use neuradock_domain::token::ApiToken;

pub struct CodexConfigService;

impl CodexConfigService {
    pub fn new() -> Self {
        Self
    }

    fn get_codex_auth_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Cannot find home directory")?;
        Ok(home.join(".codex").join("auth.json"))
    }

    /// Configure Codex globally by writing to ~/.codex/auth.json
    pub fn configure_global(&self, token: &ApiToken) -> Result<String> {
        let auth_path = Self::get_codex_auth_path()?;

        // Ensure directory exists
        if let Some(parent) = auth_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Create auth.json with API key
        let auth_content = json!({
            "OPENAI_API_KEY": token.key()
        });

        let content = serde_json::to_string_pretty(&auth_content)?;
        fs::write(&auth_path, content)?;

        Ok(format!(
            "Successfully configured Codex globally at: {}",
            auth_path.display()
        ))
    }

    /// Clear Codex global configuration
    /// Removes the auth.json file entirely since it only contains our config
    pub fn clear_global(&self) -> Result<String> {
        let auth_path = Self::get_codex_auth_path()?;

        if !auth_path.exists() {
            return Ok("No Codex configuration file found".to_string());
        }

        // Remove the auth.json file
        fs::remove_file(&auth_path)?;

        log::info!("Successfully cleared Codex configuration at: {}", auth_path.display());

        Ok("Successfully cleared Codex global configuration".to_string())
    }

    /// Generate temporary export commands for current shell session
    pub fn generate_temp_commands(&self, token: &ApiToken, base_url: &str) -> Result<String> {
        let commands = vec![
            format!("export OPENAI_API_KEY=\"{}\"", token.key()),
            format!("export AGENT_ROUTER_TOKEN=\"{}\"", token.key()),
            format!("export OPENAI_BASE_URL=\"{}\"", base_url),
            "# Run the above commands in your terminal, then start Codex".to_string(),
        ];

        Ok(commands.join("\n"))
    }
}

impl Default for CodexConfigService {
    fn default() -> Self {
        Self::new()
    }
}
