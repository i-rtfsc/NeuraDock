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
use neuradock_domain::balance_history::CheckInTrigger;
use neuradock_domain::check_in::ProviderRepository;
use neuradock_domain::proxy_config::ProxyConfigRepository;
use neuradock_domain::shared::{AccountId, DomainError};
use neuradock_domain::waf_cookies::WafCookiesRepository;

use super::shared;

/// Execute check-in command handler
pub struct ExecuteCheckInCommandHandler {
    account_repo: Arc<dyn AccountRepository>,
    provider_repo: Arc<dyn ProviderRepository>,
    proxy_config_repo: Arc<dyn ProxyConfigRepository>,
    notification_service: Option<Arc<NotificationService>>,
    provider_models_service: Arc<ProviderModelsService>,
    balance_history_service: Arc<BalanceHistoryService>,
    waf_cookies_repo: Arc<dyn WafCookiesRepository>,
    headless_browser: bool,
}

impl ExecuteCheckInCommandHandler {
    pub fn new(
        account_repo: Arc<dyn AccountRepository>,
        provider_repo: Arc<dyn ProviderRepository>,
        proxy_config_repo: Arc<dyn ProxyConfigRepository>,
        provider_models_service: Arc<ProviderModelsService>,
        balance_history_service: Arc<BalanceHistoryService>,
        waf_cookies_repo: Arc<dyn WafCookiesRepository>,
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
        .with_balance_history(self.balance_history_service.clone())
        .with_waf_cookies_repo(self.waf_cookies_repo.clone());

        // Execute check-in
        let result = executor
            .execute_check_in(&cmd.account_id, &provider, CheckInTrigger::Manual)
            .await
            .to_infra_err()?;

        info!(
            "Check-in completed for account {}: success={} outcome={:?}",
            cmd.account_id, result.success, result.outcome
        );

        // Refresh the account's balance cache when the run produced a reading.
        // The history snapshot is the executor's job.
        let balance_dto = match result.user_info.as_ref() {
            Some(user_info) if result.success => {
                let balance = shared::update_and_save_balance(
                    &self.account_repo,
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
            }
            _ => None,
        };

        // Send notification if service is available
        let balance_tuple = balance_dto
            .as_ref()
            .map(|info| (info.current_balance, info.total_consumed, info.total_quota));

        shared::send_check_in_notification(
            &self.notification_service,
            result.outcome,
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
            outcome: result.outcome,
        })
    }
}
