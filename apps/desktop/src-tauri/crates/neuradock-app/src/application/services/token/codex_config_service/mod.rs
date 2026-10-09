mod global_config;
mod helpers;
mod isolated_config;
mod model_catalog;
mod temp_commands;

use anyhow::Result;

use neuradock_domain::token::ApiToken;

pub struct CodexConfigService;

impl CodexConfigService {
    pub fn validate_profile_slug(slug: &str) -> Result<()> {
        anyhow::ensure!(
            !slug.is_empty() && slug != "default" && slug.len() <= 120
                && slug.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
                && slug.chars().next().is_some_and(|c| c.is_ascii_alphanumeric()),
            "Profile names must start with a letter or number and contain only letters, numbers, '-', '_' or '.'"
        );
        Ok(())
    }

    pub fn validate_profile_models(models: &[String], default_model: &str) -> Result<Vec<String>> {
        model_catalog::validate_models(models, default_model)
    }

    pub fn validate_model(model: &str) -> Result<()> {
        helpers::validate_codex_model(model)
    }

    pub fn profile_models(slug: &str) -> Result<Vec<String>> {
        let dir = helpers::get_codex_dir()?;
        let path = isolated_config::isolated_config_path_for(slug)?;
        let content = std::fs::read_to_string(&path)
            .or_else(|_| std::fs::read_to_string(dir.join(format!("{slug}.config.toml"))))
            .unwrap_or_default();
        let configured_path = content
            .parse::<toml_edit::DocumentMut>()
            .ok()
            .and_then(|doc| doc.get("model_catalog_json")?.as_str().map(str::to_string))
            .map(|path| {
                if let Some(relative) = path.strip_prefix("~/") {
                    dirs::home_dir().unwrap_or_default().join(relative)
                } else {
                    std::path::PathBuf::from(path)
                }
            });
        Ok(model_catalog::catalog_model_names(
            &configured_path.unwrap_or(isolated_config::model_catalog_path(slug)?),
        ))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn configure_named_profile(
        &self,
        slug: &str,
        display: &str,
        api_key: &str,
        base_url: &str,
        model: &str,
        models: &[String],
        snapshot_direct: bool,
    ) -> Result<String> {
        Self::validate_profile_slug(slug)?;
        isolated_config::configure_named_profile(
            slug,
            display,
            api_key,
            base_url,
            model,
            models,
            snapshot_direct,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn preview_named_profile(
        &self,
        slug: &str,
        display: &str,
        api_key: &str,
        base_url: &str,
        model: &str,
        models: &[String],
    ) -> Result<Vec<(std::path::PathBuf, &'static str, String, String)>> {
        Self::validate_profile_slug(slug)?;
        isolated_config::preview_named_profile(slug, display, api_key, base_url, model, models)
    }

    pub fn new() -> Self {
        Self
    }

    pub fn discard_named_gateway_snapshot(&self, slug: &str) -> Result<()> {
        isolated_config::discard_named_gateway_snapshot(slug)
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

    /// Configure Codex via a self-contained isolated file that never touches
    /// config.toml/auth.json: `~/.codex/profiles.d/<slug>.config.toml`.
    pub fn configure_isolated(
        &self,
        token: &ApiToken,
        provider_id: &str,
        provider_name: &str,
        base_url: &str,
        model: Option<&str>,
        context_1m: bool,
    ) -> Result<String> {
        isolated_config::configure_isolated_impl(
            token,
            provider_id,
            provider_name,
            base_url,
            model,
            context_1m,
        )
    }

    /// Isolated-file configuration from a raw API key (independent keys).
    pub fn configure_isolated_with_key(
        &self,
        api_key: &str,
        base_url: &str,
        model: Option<&str>,
        context_1m: bool,
    ) -> Result<String> {
        isolated_config::configure_isolated_with_key_impl(api_key, base_url, model, context_1m)
    }

    pub fn preview_isolated(
        &self,
        token: &ApiToken,
        provider_id: &str,
        provider_name: &str,
        base_url: &str,
        model: Option<&str>,
        context_1m: bool,
    ) -> Result<Vec<(std::path::PathBuf, &'static str, String, String)>> {
        isolated_config::preview_isolated_impl(
            token,
            provider_id,
            provider_name,
            base_url,
            model,
            context_1m,
        )
    }

    pub fn preview_isolated_with_key(
        &self,
        api_key: &str,
        base_url: &str,
        model: Option<&str>,
        context_1m: bool,
    ) -> Result<Vec<(std::path::PathBuf, &'static str, String, String)>> {
        isolated_config::preview_isolated_with_key_impl(api_key, base_url, model, context_1m)
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

    /// Preview the isolated files a gateway binding would write.
    pub fn preview_isolated_gateway(
        &self,
        slug_hint: &str,
        display: &str,
        gateway_base_url: &str,
        local_credential: &str,
        model: Option<&str>,
        context_1m: bool,
    ) -> Result<Vec<(std::path::PathBuf, &'static str, String, String)>> {
        isolated_config::preview_isolated_gateway_impl(
            slug_hint,
            display,
            gateway_base_url,
            local_credential,
            model,
            context_1m,
        )
    }

    /// Path of the isolated config identified by `slug_hint`.
    pub fn isolated_path(slug_hint: &str) -> Result<std::path::PathBuf> {
        isolated_config::isolated_config_path_for(slug_hint)
    }

    pub fn clear_isolated(&self, slug_hint: &str) -> Result<String> {
        isolated_config::clear_isolated_impl(slug_hint)
    }

    /// Write the gateway's isolated config: a file named after the real relay
    /// provider (`slug_hint`/`display`) whose `base_url` points at the local
    /// gateway. Never snapshots or touches the official config.toml/auth.json.
    pub fn configure_isolated_gateway(
        &self,
        slug_hint: &str,
        display: &str,
        gateway_base_url: &str,
        local_credential: &str,
        model: Option<&str>,
        context_1m: bool,
    ) -> Result<String> {
        isolated_config::configure_isolated_gateway_impl(
            slug_hint,
            display,
            gateway_base_url,
            local_credential,
            model,
            context_1m,
        )
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
