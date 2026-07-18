mod global_config;
mod helpers;
mod temp_commands;

use anyhow::Result;

use neuradock_domain::token::ApiToken;

pub struct CodexConfigService;

impl CodexConfigService {
    pub fn new() -> Self {
        Self
    }

    /// Configure Codex globally by writing to ~/.codex/config.toml and ~/.codex/auth.json
    pub fn configure_global(
        &self,
        token: &ApiToken,
        provider_id: &str,
        provider_name: &str,
        base_url: &str,
        model: Option<&str>,
        context_1m: bool,
    ) -> Result<String> {
        global_config::configure_global_impl(
            token,
            provider_id,
            provider_name,
            base_url,
            model,
            context_1m,
        )
    }

    /// Configure Codex globally with API key string (for independent keys)
    pub fn configure_global_with_key(
        &self,
        api_key: &str,
        base_url: &str,
        model: Option<&str>,
        context_1m: bool,
    ) -> Result<String> {
        global_config::configure_global_with_key_impl(api_key, base_url, model, context_1m)
    }

    pub fn preview_global(
        &self,
        token: &ApiToken,
        provider_id: &str,
        provider_name: &str,
        base_url: &str,
        model: Option<&str>,
        context_1m: bool,
    ) -> Result<Vec<(std::path::PathBuf, &'static str, String, String)>> {
        global_config::preview_global_impl(
            token,
            provider_id,
            provider_name,
            base_url,
            model,
            context_1m,
        )
    }

    pub fn preview_global_with_key(
        &self,
        api_key: &str,
        base_url: &str,
        model: Option<&str>,
        context_1m: bool,
    ) -> Result<Vec<(std::path::PathBuf, &'static str, String, String)>> {
        global_config::preview_global_with_key_impl(api_key, base_url, model, context_1m)
    }

    /// Clear Codex global configuration
    /// Removes both config.toml and auth.json files
    pub fn clear_global(&self) -> Result<String> {
        global_config::clear_global_impl()
    }

    /// Paths of the files this service manages: (config.toml, auth.json).
    pub fn managed_paths() -> Result<(std::path::PathBuf, std::path::PathBuf)> {
        Ok((
            helpers::get_codex_config_path()?,
            helpers::get_codex_auth_path()?,
        ))
    }

    /// Generate temporary export commands for current shell session
    /// Note: This feature is temporarily unavailable
    #[allow(dead_code)]
    pub fn generate_temp_commands(
        &self,
        token: &ApiToken,
        provider_id: &str,
        provider_name: &str,
        base_url: &str,
        model: Option<&str>,
        context_1m: bool,
    ) -> Result<String> {
        temp_commands::generate_temp_commands_impl(
            token,
            provider_id,
            provider_name,
            base_url,
            model,
            context_1m,
        )
    }

    /// Generate temporary export commands with API key string (for independent keys)
    #[allow(dead_code)]
    pub fn generate_temp_commands_with_key(
        &self,
        api_key: &str,
        base_url: &str,
        model: Option<&str>,
        context_1m: bool,
    ) -> Result<String> {
        temp_commands::generate_temp_commands_with_key_impl(api_key, base_url, model, context_1m)
    }
}

impl Default for CodexConfigService {
    fn default() -> Self {
        Self::new()
    }
}
