use async_trait::async_trait;
use log::info;
use std::sync::Arc;

use crate::application::commands::check_in_commands::*;
use crate::application::commands::command_handler::CommandHandler;
use crate::application::services::{
    BalanceHistoryService, CheckInExecutor, NotificationService, ProviderModelsService,
};
use crate::application::ResultExt;
use neuradock_domain::account::AccountRepository;
use neuradock_domain::check_in::ProviderRepository;
use neuradock_domain::proxy_config::ProxyConfigRepository;
use neuradock_domain::shared::{AccountId, DomainError};
use neuradock_infrastructure::persistence::repositories::SqliteWafCookiesRepository;

use super::shared;

/// Execute check-in command handler
pub struct ExecuteCheckInCommandHandler {
    account_repo: Arc<dyn AccountRepository>,
    provider_repo: Arc<dyn ProviderRepository>,
    proxy_config_repo: Arc<dyn ProxyConfigRepository>,
    notification_service: Option<Arc<NotificationService>>,
    provider_models_service: Arc<ProviderModelsService>,
    balance_history_service: Arc<BalanceHistoryService>,
    waf_cookies_repo: Arc<SqliteWafCookiesRepository>,
    headless_browser: bool,
}

impl ExecuteCheckInCommandHandler {
    pub fn new(
        account_repo: Arc<dyn AccountRepository>,
        provider_repo: Arc<dyn ProviderRepository>,
        proxy_config_repo: Arc<dyn ProxyConfigRepository>,
        provider_models_service: Arc<ProviderModelsService>,
        balance_history_service: Arc<BalanceHistoryService>,
        waf_cookies_repo: Arc<SqliteWafCookiesRepository>,
        headless_browser: bool,
    ) -> Self {
        Self {
            account_repo,
            provider_repo,
            proxy_config_repo,
            notification_service: None,
            provider_models_service,
            balance_history_service,
            waf_cookies_repo,
            headless_browser,
        }
    }

    pub fn with_notification_service(mut self, service: Arc<NotificationService>) -> Self {
        self.notification_service = Some(service);
        self
    }
}

#[async_trait]
impl CommandHandler<ExecuteCheckInCommand> for ExecuteCheckInCommandHandler {
    type Result = CheckInCommandResult;

    async fn handle(&self, cmd: ExecuteCheckInCommand) -> Result<Self::Result, DomainError> {
        info!(
            "Handling ExecuteCheckInCommand for account: {}",
            cmd.account_id
        );

        // Load account to get provider_id
        let account = self
            .account_repo
            .find_by_id(&AccountId::from_string(&cmd.account_id))
            .await?
            .ok_or_else(|| {
                DomainError::AccountNotFound(format!("Account not found: {}", cmd.account_id))
            })?;

        // Get provider from account's provider_id
        let provider_id = account.provider_id().as_str().to_string();
        let provider = self
            .provider_repo
            .find_by_id(account.provider_id())
            .await?
            .ok_or_else(|| {
                DomainError::ProviderNotFound(format!("Provider not found: {}", provider_id))
            })?;

        let account_name = account.name().to_string();

        // Get proxy configuration
        let proxy_config = self.proxy_config_repo.get().await?;
        let proxy_url = proxy_config.proxy_url();

        // Create executor with proxy support
        let executor = CheckInExecutor::with_proxy(
            self.account_repo.clone(),
            self.headless_browser,
            proxy_url,
        )
        .to_infra_err()?
        .with_waf_cookies_repo(self.waf_cookies_repo.clone());

        // Execute check-in
        let result = executor
            .execute_check_in(&cmd.account_id, &provider)
            .await
            .to_infra_err()?;

        info!(
            "Check-in completed for account {}: success={}",
            cmd.account_id, result.success
        );

        // Update account balance cache and save to balance_history if we have new balance data
        let balance_dto = if result.success && result.user_info.is_some() {
            let user_info = match result.user_info.as_ref() {
                Some(info) => info,
                None => {
                    log::warn!("Check-in succeeded but no user_info returned");
                    return Err(DomainError::Validation(
                        "Check-in response missing user_info".to_string(),
                    ));
                }
            };

            let balance = shared::update_and_save_balance(
                &self.account_repo,
                &self.balance_history_service,
                &cmd.account_id,
                account,
                user_info,
            )
            .await?;

            // Auto-fetch provider models if not exists in database
            shared::auto_fetch_provider_models(
                &self.account_repo,
                &self.provider_models_service,
                &cmd.account_id,
                &provider,
            )
            .await;

            Some(balance)
        } else {
            None
        };

        // Send notification if service is available
        let balance_tuple = result
            .user_info
            .as_ref()
            .map(|info| (info.quota, info.used_quota, info.quota + info.used_quota));

        shared::send_check_in_notification(
            &self.notification_service,
            result.success,
            &cmd.account_id,
            &account_name,
            provider.name(),
            &result.message,
            balance_tuple,
        )
        .await;

        Ok(CheckInCommandResult {
            account_id: cmd.account_id,
            account_name,
            provider_id,
            success: result.success,
            message: result.message,
            balance: balance_dto,
        })
    }
}
