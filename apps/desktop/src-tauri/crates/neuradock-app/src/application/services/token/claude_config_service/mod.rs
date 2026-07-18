mod global_config;
mod helpers;
mod temp_commands;

use anyhow::Result;

use neuradock_domain::token::ApiToken;

pub struct ClaudeConfigService;

impl ClaudeConfigService {
    pub fn new() -> Self {
        Self
    }

    /// Model id sent by Claude Code when the 1M context variant is selected.
    /// Configure Claude Code globally by writing to ~/.claude/settings.json
    /// This properly merges with existing configuration
    pub fn configure_global(
        &self,
        token: &ApiToken,
        base_url: &str,
        model: Option<&str>,
        context_1m: bool,
    ) -> Result<String> {
        global_config::configure_global_impl(token, base_url, model, context_1m)
    }

    /// Configure Claude Code globally with API key string (for independent keys)
    pub fn configure_global_with_key(
        &self,
        api_key: &str,
        base_url: &str,
        model: Option<&str>,
        context_1m: bool,
    ) -> Result<String> {
        global_config::configure_global_with_key_impl(api_key, base_url, model, context_1m)
    }

    pub fn preview_global_with_key(
        &self,
        api_key: &str,
        base_url: &str,
        model: Option<&str>,
        context_1m: bool,
    ) -> Result<(std::path::PathBuf, String, String)> {
        global_config::preview_global_with_key_impl(api_key, base_url, model, context_1m)
    }

    /// Path of the settings.json this service manages.
    pub fn managed_path() -> Result<std::path::PathBuf> {
        helpers::get_claude_config_path()
    }

    /// Clear Claude Code global configuration
    /// Only removes the env keys that we manage, preserves other settings
    pub fn clear_global(&self) -> Result<String> {
        global_config::clear_global_impl()
    }

    /// Generate temporary export commands for current shell session
    pub fn generate_temp_commands(
        &self,
        token: &ApiToken,
        base_url: &str,
        model: Option<&str>,
        context_1m: bool,
    ) -> Result<String> {
        temp_commands::generate_temp_commands_impl(token, base_url, model, context_1m)
    }

    /// Generate temporary export commands with API key string (for independent keys)
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

impl Default for ClaudeConfigService {
    fn default() -> Self {
        Self::new()
    }
}
