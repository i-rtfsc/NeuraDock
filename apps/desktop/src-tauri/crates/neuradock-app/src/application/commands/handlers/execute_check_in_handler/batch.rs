use async_trait::async_trait;
use log::{error, info};
use std::sync::Arc;

use crate::application::commands::check_in_commands::*;
use crate::application::commands::command_handler::CommandHandler;
use crate::application::dtos::BalanceDto;
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

/// Batch execute check-in command handler
pub struct BatchExecuteCheckInCommandHandler {
    account_repo: Arc<dyn AccountRepository>,
    provider_repo: Arc<dyn ProviderRepository>,
    proxy_config_repo: Arc<dyn ProxyConfigRepository>,
    notification_service: Option<Arc<NotificationService>>,
    provider_models_service: Arc<ProviderModelsService>,
    balance_history_service: Arc<BalanceHistoryService>,
    waf_cookies_repo: Arc<SqliteWafCookiesRepository>,
    headless_browser: bool,
}

impl BatchExecuteCheckInCommandHandler {
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
impl CommandHandler<BatchExecuteCheckInCommand> for BatchExecuteCheckInCommandHandler {
    type Result = BatchCheckInCommandResult;

    async fn handle(&self, cmd: BatchExecuteCheckInCommand) -> Result<Self::Result, DomainError> {
        info!(
            "Handling BatchExecuteCheckInCommand for {} accounts",
            cmd.account_ids.len()
        );

        let total = cmd.account_ids.len();
        let mut succeeded = 0;
        let mut failed = 0;
        let mut results = Vec::new();

        // Get proxy configuration
        let proxy_config = self.proxy_config_repo.get().await?;
        let proxy_url = proxy_config.proxy_url();

        let executor = CheckInExecutor::with_proxy(
            self.account_repo.clone(),
            self.headless_browser,
            proxy_url,
        )
        .to_infra_err()?
        .with_waf_cookies_repo(self.waf_cookies_repo.clone());

        for account_id in cmd.account_ids {
            // Load account to get provider_id
            let account = match self
                .account_repo
                .find_by_id(&AccountId::from_string(&account_id))
                .await
            {
                Ok(Some(acc)) => acc,
                Ok(None) => {
                    error!("Account not found: {}", account_id);
                    failed += 1;
                    results.push(CheckInCommandResult {
                        account_id: account_id.clone(),
                        account_name: String::new(),
                        provider_id: String::new(),
                        success: false,
                        message: format!("Account not found: {}", account_id),
                        balance: None,
                    });
                    continue;
                }
                Err(e) => {
                    error!("Failed to load account {}: {}", account_id, e);
                    failed += 1;
                    results.push(CheckInCommandResult {
                        account_id: account_id.clone(),
                        account_name: String::new(),
                        provider_id: String::new(),
                        success: false,
                        message: format!("Failed to load account: {}", e),
                        balance: None,
                    });
                    continue;
                }
            };

            // Get provider from account's provider_id
            let provider_id = account.provider_id().as_str().to_string();
            let account_name = account.name().to_string();
            let provider = match self.provider_repo.find_by_id(account.provider_id()).await {
                Ok(Some(provider)) => provider,
                Ok(None) => {
                    error!("Provider not found: {}", provider_id);
                    failed += 1;
                    results.push(CheckInCommandResult {
                        account_id: account_id.clone(),
                        account_name,
                        provider_id: provider_id.clone(),
                        success: false,
                        message: format!("Provider not found: {}", provider_id),
                        balance: None,
                    });
                    continue;
                }
                Err(e) => {
                    error!("Failed to load provider {}: {}", provider_id, e);
                    failed += 1;
                    results.push(CheckInCommandResult {
                        account_id: account_id.clone(),
                        account_name,
                        provider_id: provider_id.clone(),
                        success: false,
                        message: format!("Failed to load provider {}: {}", provider_id, e),
                        balance: None,
                    });
                    continue;
                }
            };

            match executor.execute_check_in(&account_id, &provider).await {
                Ok(result) => {
                    // Update account balance cache and save to balance_history if we have new balance data
                    let balance_dto = if result.success && result.user_info.is_some() {
                        match shared::update_and_save_balance(
                            &self.account_repo,
                            &self.balance_history_service,
                            &account_id,
                            account,
                            result.user_info.as_ref().unwrap(),
                        )
                        .await
                        {
                            Ok(balance) => {
                                // Auto-fetch provider models if not exists in database
                                shared::auto_fetch_provider_models(
                                    &self.account_repo,
                                    &self.provider_models_service,
                                    &account_id,
                                    &provider,
                                )
                                .await;

                                Some(balance)
                            }
                            Err(e) => {
                                error!(
                                    "Failed to update balance for account {}: {}",
                                    account_id, e
                                );
                                None
                            }
                        }
                    } else {
                        result.user_info.as_ref().map(|info| BalanceDto {
                            current_balance: info.quota,
                            total_consumed: info.used_quota,
                            total_income: info.quota + info.used_quota,
                        })
                    };

                    // Send notification if service is available
                    let balance_tuple = result
                        .user_info
                        .as_ref()
                        .map(|info| (info.quota, info.used_quota, info.quota + info.used_quota));

                    shared::send_check_in_notification(
                        &self.notification_service,
                        result.success,
                        &account_id,
                        &result.account_name,
                        provider.name(),
                        &result.message,
                        balance_tuple,
                    )
                    .await;

                    if result.success {
                        succeeded += 1;
                    } else {
                        failed += 1;
                    }
                    results.push(CheckInCommandResult {
                        account_id: account_id.clone(),
                        account_name: account_name.clone(),
                        provider_id: provider_id.clone(),
                        success: result.success,
                        message: result.message,
                        balance: balance_dto,
                    });
                }
                Err(e) => {
                    error!("Check-in failed for account {}: {}", account_id, e);
                    failed += 1;
                    results.push(CheckInCommandResult {
                        account_id: account_id.clone(),
                        account_name: account_name.clone(),
                        provider_id: provider_id.clone(),
                        success: false,
                        message: format!("Check-in failed: {}", e),
                        balance: None,
                    });
                }
            }
        }

        info!(
            "Batch check-in completed: total={}, succeeded={}, failed={}",
            total, succeeded, failed
        );

        Ok(BatchCheckInCommandResult {
            total,
            succeeded,
            failed,
            results,
        })
    }
}
