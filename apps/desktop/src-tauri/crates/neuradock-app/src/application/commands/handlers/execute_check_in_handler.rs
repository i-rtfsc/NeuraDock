use async_trait::async_trait;
use log::{error, info};
use std::sync::Arc;

use crate::application::commands::check_in_commands::*;
use crate::application::commands::command_handler::CommandHandler;
use crate::application::dtos::BalanceDto;
use crate::application::services::CheckInExecutor;
use neuradock_domain::account::AccountRepository;
use neuradock_domain::check_in::Provider;
use neuradock_domain::shared::DomainError;

/// Execute check-in command handler
pub struct ExecuteCheckInCommandHandler {
    account_repo: Arc<dyn AccountRepository>,
    headless_browser: bool,
}

impl ExecuteCheckInCommandHandler {
    pub fn new(account_repo: Arc<dyn AccountRepository>, headless_browser: bool) -> Self {
        Self {
            account_repo,
            headless_browser,
        }
    }
}

#[async_trait]
impl CommandHandler<ExecuteCheckInCommand> for ExecuteCheckInCommandHandler {
    type Result = CheckInCommandResult;

    async fn handle(&self, cmd: ExecuteCheckInCommand) -> Result<Self::Result, DomainError> {
        info!("Handling ExecuteCheckInCommand for account: {}", cmd.account_id);

        // Create executor
        let executor = CheckInExecutor::new(self.account_repo.clone(), self.headless_browser)
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;

        // Get provider (this should be passed or loaded properly)
        // For now, we'll need to get it from somewhere
        // TODO: This is a temporary solution, should be refactored
        let provider = self.get_provider_for_account(&cmd.account_id).await?;

        // Execute check-in
        let result = executor
            .execute_check_in(&cmd.account_id, &provider)
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

impl ExecuteCheckInCommandHandler {
    async fn get_provider_for_account(&self, _account_id: &str) -> Result<Provider, DomainError> {
        // TODO: This is a temporary implementation
        // Should load provider from account's provider_id
        // For now, return a default provider
        Ok(Provider::builtin(
            "anyrouter",
            "AnyRouter".to_string(),
            "https://anyrouter.top".to_string(),
            "/login".to_string(),
            Some("/api/user/sign_in".to_string()),
            "/api/user/self".to_string(),
            "new-api-user".to_string(),
            Some("waf_cookies".to_string()),
        ))
    }
}

/// Batch execute check-in command handler
pub struct BatchExecuteCheckInCommandHandler {
    account_repo: Arc<dyn AccountRepository>,
    headless_browser: bool,
}

impl BatchExecuteCheckInCommandHandler {
    pub fn new(account_repo: Arc<dyn AccountRepository>, headless_browser: bool) -> Self {
        Self {
            account_repo,
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
            let provider = match self.get_provider_for_account(&account_id).await {
                Ok(p) => p,
                Err(e) => {
                    error!("Failed to get provider for account {}: {}", account_id, e);
                    failed += 1;
                    results.push(CheckInCommandResult {
                        success: false,
                        message: format!("Failed to get provider: {}", e),
                        balance: None,
                    });
                    continue;
                }
            };

            match executor.execute_check_in(&account_id, &provider).await {
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

impl BatchExecuteCheckInCommandHandler {
    async fn get_provider_for_account(&self, _account_id: &str) -> Result<Provider, DomainError> {
        // TODO: Same as single check-in handler
        Ok(Provider::builtin(
            "anyrouter",
            "AnyRouter".to_string(),
            "https://anyrouter.top".to_string(),
            "/login".to_string(),
            Some("/api/user/sign_in".to_string()),
            "/api/user/self".to_string(),
            "new-api-user".to_string(),
            Some("waf_cookies".to_string()),
        ))
    }
}
