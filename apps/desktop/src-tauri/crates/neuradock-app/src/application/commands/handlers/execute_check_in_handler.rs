use async_trait::async_trait;
use log::{error, info};
use std::collections::HashMap;
use std::sync::Arc;

use crate::application::commands::check_in_commands::*;
use crate::application::commands::command_handler::CommandHandler;
use crate::application::dtos::BalanceDto;
use crate::application::services::CheckInExecutor;
use neuradock_domain::account::AccountRepository;
use neuradock_domain::check_in::Provider;
use neuradock_domain::shared::{AccountId, DomainError};

/// Execute check-in command handler
pub struct ExecuteCheckInCommandHandler {
    account_repo: Arc<dyn AccountRepository>,
    providers: HashMap<String, Provider>,
    headless_browser: bool,
}

impl ExecuteCheckInCommandHandler {
    pub fn new(
        account_repo: Arc<dyn AccountRepository>,
        providers: HashMap<String, Provider>,
        headless_browser: bool,
    ) -> Self {
        Self {
            account_repo,
            providers,
            headless_browser,
        }
    }
}

#[async_trait]
impl CommandHandler<ExecuteCheckInCommand> for ExecuteCheckInCommandHandler {
    type Result = CheckInCommandResult;

    async fn handle(&self, cmd: ExecuteCheckInCommand) -> Result<Self::Result, DomainError> {
        info!("Handling ExecuteCheckInCommand for account: {}", cmd.account_id);

        // Load account to get provider_id
        let account = self
            .account_repo
            .find_by_id(&AccountId::from_string(&cmd.account_id))
            .await?
            .ok_or_else(|| {
                DomainError::AccountNotFound(format!("Account not found: {}", cmd.account_id))
            })?;

        // Get provider from account's provider_id
        let provider_id = account.provider_id().as_str();
        let provider = self.providers.get(provider_id).ok_or_else(|| {
            DomainError::ProviderNotFound(format!("Provider not found: {}", provider_id))
        })?;

        // Create executor
        let executor = CheckInExecutor::new(self.account_repo.clone(), self.headless_browser)
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;

        // Execute check-in
        let result = executor
            .execute_check_in(&cmd.account_id, provider)
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;

        info!(
            "Check-in completed for account {}: success={}",
            cmd.account_id, result.success
        );

        Ok(CheckInCommandResult {
            success: result.success,
            message: result.message,
            balance: result.user_info.map(|info| BalanceDto {
                current_balance: info.quota,
                total_consumed: info.used_quota,
                total_income: info.quota + info.used_quota,
            }),
        })
    }
}

/// Batch execute check-in command handler
pub struct BatchExecuteCheckInCommandHandler {
    account_repo: Arc<dyn AccountRepository>,
    providers: HashMap<String, Provider>,
    headless_browser: bool,
}

impl BatchExecuteCheckInCommandHandler {
    pub fn new(
        account_repo: Arc<dyn AccountRepository>,
        providers: HashMap<String, Provider>,
        headless_browser: bool,
    ) -> Self {
        Self {
            account_repo,
            providers,
            headless_browser,
        }
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

        let executor = CheckInExecutor::new(self.account_repo.clone(), self.headless_browser)
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;

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
                        success: false,
                        message: format!("Failed to load account: {}", e),
                        balance: None,
                    });
                    continue;
                }
            };

            // Get provider from account's provider_id
            let provider_id = account.provider_id().as_str();
            let provider = match self.providers.get(provider_id) {
                Some(p) => p,
                None => {
                    error!("Provider not found: {}", provider_id);
                    failed += 1;
                    results.push(CheckInCommandResult {
                        success: false,
                        message: format!("Provider not found: {}", provider_id),
                        balance: None,
                    });
                    continue;
                }
            };

            match executor.execute_check_in(&account_id, provider).await {
                Ok(result) => {
                    if result.success {
                        succeeded += 1;
                    } else {
                        failed += 1;
                    }
                    results.push(CheckInCommandResult {
                        success: result.success,
                        message: result.message,
                        balance: result.user_info.map(|info| BalanceDto {
                            current_balance: info.quota,
                            total_consumed: info.used_quota,
                            total_income: info.quota + info.used_quota,
                        }),
                    });
                }
                Err(e) => {
                    error!("Check-in failed for account {}: {}", account_id, e);
                    failed += 1;
                    results.push(CheckInCommandResult {
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
