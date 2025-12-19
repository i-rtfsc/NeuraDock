use async_trait::async_trait;
use log::info;
use std::sync::Arc;

use crate::application::commands::command_handler::CommandHandler;
use crate::application::commands::provider_commands::*;
use neuradock_domain::check_in::{Provider, ProviderConfig, ProviderRepository};
use neuradock_domain::shared::DomainError;

/// Create provider command handler
pub struct CreateProviderCommandHandler {
    provider_repo: Arc<dyn ProviderRepository>,
}

impl CreateProviderCommandHandler {
    pub fn new(provider_repo: Arc<dyn ProviderRepository>) -> Self {
        Self { provider_repo }
    }
}

#[async_trait]
impl CommandHandler<CreateProviderCommand> for CreateProviderCommandHandler {
    type Result = CreateProviderResult;

    async fn handle(&self, cmd: CreateProviderCommand) -> Result<Self::Result, DomainError> {
        info!(
            "🔵 Handling CreateProviderCommand for provider: {}",
            cmd.name
        );

        // Validate domain (should start with http:// or https://)
        if !cmd.domain.starts_with("http://") && !cmd.domain.starts_with("https://") {
            return Err(DomainError::Validation(
                "Domain must start with http:// or https://".to_string(),
            ));
        }

        let supports_check_in = cmd.supports_check_in.unwrap_or(true);
        let check_in_bugged = cmd.check_in_bugged.unwrap_or(false);

        // Use provided values or new-api defaults
        let provider = Provider::new(ProviderConfig {
            name: cmd.name.clone(),
            domain: cmd.domain.clone(),
            login_path: cmd.login_path.unwrap_or_else(|| "/login".to_string()),
            sign_in_path: cmd.sign_in_path.or(Some("/api/user/sign_in".to_string())),
            user_info_path: cmd
                .user_info_path
                .unwrap_or_else(|| "/api/user/self".to_string()),
            token_api_path: cmd.token_api_path.or(Some("/api/token/".to_string())),
            models_path: cmd.models_path.or(Some("/api/user/models".to_string())),
            api_user_key: cmd
                .api_user_key
                .unwrap_or_else(|| "new-api-user".to_string()),
            bypass_method: if cmd.needs_waf_bypass {
                Some("waf_cookies".to_string())
            } else {
                None
            },
            supports_check_in,
            check_in_bugged,
        });

        let provider_id = provider.id().as_str().to_string();
        info!("🆕 Creating provider with ID: {}", provider_id);

        // Save provider
        self.provider_repo.save(&provider).await?;

        info!(
            "✅ Provider created successfully: {} ({})",
            cmd.name, provider_id
        );

        Ok(CreateProviderResult { provider_id })
    }
}

/// Update provider command handler
pub struct UpdateProviderCommandHandler {
    provider_repo: Arc<dyn ProviderRepository>,
}

impl UpdateProviderCommandHandler {
    pub fn new(provider_repo: Arc<dyn ProviderRepository>) -> Self {
        Self { provider_repo }
    }
}

#[async_trait]
impl CommandHandler<UpdateProviderCommand> for UpdateProviderCommandHandler {
    type Result = UpdateProviderResult;

    async fn handle(&self, cmd: UpdateProviderCommand) -> Result<Self::Result, DomainError> {
        info!(
            "Handling UpdateProviderCommand for provider: {}",
            cmd.provider_id
        );

        // Find existing provider
        let provider_id = neuradock_domain::shared::ProviderId::from_string(&cmd.provider_id);
        let existing = self
            .provider_repo
            .find_by_id(&provider_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Provider not found".to_string()))?;

        // Validate domain if provided
        if let Some(ref domain) = cmd.domain {
            if !domain.starts_with("http://") && !domain.starts_with("https://") {
                return Err(DomainError::Validation(
                    "Domain must start with http:// or https://".to_string(),
                ));
            }
        }

        // Extract current values as owned strings to avoid lifetime issues
        let current_login_path = existing
            .login_url()
            .trim_start_matches(existing.domain())
            .to_string();
        let current_sign_in_path = existing
            .sign_in_url()
            .as_ref()
            .map(|url| url.trim_start_matches(existing.domain()).to_string());
        let current_user_info_path = existing
            .user_info_url()
            .trim_start_matches(existing.domain())
            .to_string();
        let current_token_api_path = existing
            .token_api_url()
            .as_ref()
            .map(|url| url.trim_start_matches(existing.domain()).to_string());
        let current_models_path = existing
            .models_url()
            .as_ref()
            .map(|url| url.trim_start_matches(existing.domain()).to_string());
        let current_name = existing.name().to_string();
        let current_domain = existing.domain().to_string();
        let current_api_user_key = existing.api_user_key().to_string();
        let current_needs_waf = existing.needs_waf_bypass();
        let current_supports_check_in = existing.supports_check_in();
        let current_check_in_bugged = existing.check_in_bugged();
        let current_is_builtin = existing.is_builtin();
        let current_created_at = existing.created_at();

        // Create updated provider using provided values or existing values as fallback
        // IMPORTANT: Use restore() to preserve the existing provider ID, is_builtin, and created_at
        let updated_provider = Provider::restore(
            provider_id.clone(),
            ProviderConfig {
                name: cmd.name.unwrap_or(current_name),
                domain: cmd.domain.unwrap_or(current_domain),
                login_path: cmd.login_path.unwrap_or(current_login_path),
                sign_in_path: cmd.sign_in_path.or(current_sign_in_path),
                user_info_path: cmd.user_info_path.unwrap_or(current_user_info_path),
                token_api_path: cmd.token_api_path.or(current_token_api_path),
                models_path: cmd.models_path.or(current_models_path),
                api_user_key: cmd.api_user_key.unwrap_or(current_api_user_key),
                bypass_method: if cmd.needs_waf_bypass.unwrap_or(current_needs_waf) {
                    Some("waf_cookies".to_string())
                } else {
                    None
                },
                supports_check_in: cmd
                    .supports_check_in
                    .unwrap_or(current_supports_check_in),
                check_in_bugged: cmd.check_in_bugged.unwrap_or(current_check_in_bugged),
            },
            current_is_builtin,
            current_created_at,
        );

        // Save updated provider
        self.provider_repo.save(&updated_provider).await?;

        info!("Provider updated successfully: {}", cmd.provider_id);

        Ok(UpdateProviderResult { success: true })
    }
}

/// Delete provider command handler
pub struct DeleteProviderCommandHandler {
    provider_repo: Arc<dyn ProviderRepository>,
}

impl DeleteProviderCommandHandler {
    pub fn new(provider_repo: Arc<dyn ProviderRepository>) -> Self {
        Self { provider_repo }
    }
}

#[async_trait]
impl CommandHandler<DeleteProviderCommand> for DeleteProviderCommandHandler {
    type Result = DeleteProviderResult;

    async fn handle(&self, cmd: DeleteProviderCommand) -> Result<Self::Result, DomainError> {
        info!(
            "Handling DeleteProviderCommand for provider: {}",
            cmd.provider_id
        );

        let provider_id = neuradock_domain::shared::ProviderId::from_string(&cmd.provider_id);

        // Delete will fail if provider is builtin (checked in repository)
        self.provider_repo.delete(&provider_id).await?;

        info!("Provider deleted successfully: {}", cmd.provider_id);

        Ok(DeleteProviderResult { success: true })
    }
}
