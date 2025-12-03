use async_trait::async_trait;
use chrono::Utc;
use log::{error, info};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::application::commands::check_in_commands::*;
use crate::application::commands::command_handler::CommandHandler;
use crate::application::dtos::BalanceDto;
use crate::application::services::{CheckInExecutor, NotificationService};
use neuradock_domain::account::AccountRepository;
use neuradock_domain::check_in::Provider;
use neuradock_domain::shared::{AccountId, DomainError};

/// Execute check-in command handler
pub struct ExecuteCheckInCommandHandler {
    account_repo: Arc<dyn AccountRepository>,
    notification_service: Option<Arc<NotificationService>>,
    providers: HashMap<String, Provider>,
    headless_browser: bool,
    pool: Arc<SqlitePool>,
}

impl ExecuteCheckInCommandHandler {
    pub fn new(
        account_repo: Arc<dyn AccountRepository>,
        providers: HashMap<String, Provider>,
        headless_browser: bool,
        pool: Arc<SqlitePool>,
    ) -> Self {
        Self {
            account_repo,
            notification_service: None,
            providers,
            headless_browser,
            pool,
        }
    }

    pub fn with_notification_service(mut self, service: Arc<NotificationService>) -> Self {
        self.notification_service = Some(service);
        self
    }

    /// Save balance to balance_history table (one record per day, always update if exists)
    async fn save_balance_history(&self, account_id: &str, balance: &BalanceDto) {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Check if we already have a record today
        let existing: Result<Option<(String,)>, _> = sqlx::query_as(
            "SELECT id FROM balance_history WHERE account_id = ? AND DATE(recorded_at) = DATE(?) LIMIT 1",
        )
        .bind(account_id)
        .bind(now)
        .fetch_optional(&*self.pool)
        .await;

        match existing {
            Ok(Some((existing_id,))) => {
                // Record exists for today - always update with latest values
                let result = sqlx::query(
                    "UPDATE balance_history
                     SET current_balance = ?, total_consumed = ?, total_income = ?, recorded_at = ?
                     WHERE id = ?"
                )
                .bind(balance.current_balance)
                .bind(balance.total_consumed)
                .bind(balance.total_income)
                .bind(now)
                .bind(&existing_id)
                .execute(&*self.pool)
                .await;

                match result {
                    Ok(_) => {
                        info!("Balance history updated for account {}: current=${:.2}, consumed=${:.2}, income=${:.2}",
                            account_id, balance.current_balance, balance.total_consumed, balance.total_income);
                    }
                    Err(e) => {
                        error!("Failed to update balance history: {}", e);
                    }
                }
            }
            Ok(None) => {
                // Insert new record
                let result = sqlx::query(
                    "INSERT INTO balance_history (id, account_id, current_balance, total_consumed, total_income, recorded_at)
                     VALUES (?, ?, ?, ?, ?, ?)"
                )
                .bind(&id)
                .bind(account_id)
                .bind(balance.current_balance)
                .bind(balance.total_consumed)
                .bind(balance.total_income)
                .bind(now)
                .execute(&*self.pool)
                .await;

                match result {
                    Ok(_) => {
                        info!("Balance history saved for account {}: current=${:.2}, consumed=${:.2}, income=${:.2}",
                            account_id, balance.current_balance, balance.total_consumed, balance.total_income);
                    }
                    Err(e) => {
                        error!("Failed to save balance history: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to check balance history: {}", e);
            }
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

        let account_name = account.name().to_string();

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

        // Update account balance cache and save to balance_history if we have new balance data
        let balance_dto = if result.success && result.user_info.is_some() {
            let user_info = result.user_info.as_ref().unwrap();
            let mut updated_account = account;
            updated_account.update_balance(
                user_info.quota,
                user_info.used_quota,
                user_info.quota + user_info.used_quota,
            );

            // Save updated account with new balance
            if let Err(e) = self.account_repo.save(&updated_account).await {
                error!("Failed to save account balance after check-in: {}", e);
            } else {
                info!("Account balance updated in database: current=${:.2}, consumed=${:.2}, income=${:.2}",
                    user_info.quota, user_info.used_quota, user_info.quota + user_info.used_quota);
            }

            // Build balance DTO
            let balance = BalanceDto {
                current_balance: user_info.quota,
                total_consumed: user_info.used_quota,
                total_income: user_info.quota + user_info.used_quota,
            };

            // Save to balance_history table
            self.save_balance_history(&cmd.account_id, &balance).await;

            Some(balance)
        } else {
            None
        };

        // Send notification if service is available
        if let Some(notification_service) = &self.notification_service {
            let provider_name = provider.name();

            if result.success {
                // Send success notification
                let balance = result.user_info.as_ref().map(|info| {
                    (info.quota, info.used_quota, info.quota + info.used_quota)
                });

                if let Err(e) = notification_service
                    .send_check_in_success(&cmd.account_id, &account_name, provider_name, balance)
                    .await
                {
                    error!("Failed to send check-in success notification: {}", e);
                } else {
                    info!("Check-in success notification sent");
                }
            } else {
                // Send failure notification
                if let Err(e) = notification_service
                    .send_check_in_failure(&account_name, provider_name, &result.message)
                    .await
                {
                    error!("Failed to send check-in failure notification: {}", e);
                } else {
                    info!("Check-in failure notification sent");
                }
            }
        } else {
            info!("Notification service not available, skipping notification");
        }

        Ok(CheckInCommandResult {
            success: result.success,
            message: result.message,
            balance: balance_dto,
        })
    }
}

/// Batch execute check-in command handler
pub struct BatchExecuteCheckInCommandHandler {
    account_repo: Arc<dyn AccountRepository>,
    notification_service: Option<Arc<NotificationService>>,
    providers: HashMap<String, Provider>,
    headless_browser: bool,
    pool: Arc<SqlitePool>,
}

impl BatchExecuteCheckInCommandHandler {
    pub fn new(
        account_repo: Arc<dyn AccountRepository>,
        providers: HashMap<String, Provider>,
        headless_browser: bool,
        pool: Arc<SqlitePool>,
    ) -> Self {
        Self {
            account_repo,
            notification_service: None,
            providers,
            headless_browser,
            pool,
        }
    }

    pub fn with_notification_service(mut self, service: Arc<NotificationService>) -> Self {
        self.notification_service = Some(service);
        self
    }

    /// Save balance to balance_history table (one record per day, always update if exists)
    async fn save_balance_history(&self, account_id: &str, balance: &BalanceDto) {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Check if we already have a record today
        let existing: Result<Option<(String,)>, _> = sqlx::query_as(
            "SELECT id FROM balance_history WHERE account_id = ? AND DATE(recorded_at) = DATE(?) LIMIT 1",
        )
        .bind(account_id)
        .bind(now)
        .fetch_optional(&*self.pool)
        .await;

        match existing {
            Ok(Some((existing_id,))) => {
                // Record exists for today - always update with latest values
                let result = sqlx::query(
                    "UPDATE balance_history
                     SET current_balance = ?, total_consumed = ?, total_income = ?, recorded_at = ?
                     WHERE id = ?"
                )
                .bind(balance.current_balance)
                .bind(balance.total_consumed)
                .bind(balance.total_income)
                .bind(now)
                .bind(&existing_id)
                .execute(&*self.pool)
                .await;

                match result {
                    Ok(_) => {
                        info!("Balance history updated for account {}: current=${:.2}, consumed=${:.2}, income=${:.2}",
                            account_id, balance.current_balance, balance.total_consumed, balance.total_income);
                    }
                    Err(e) => {
                        error!("Failed to update balance history: {}", e);
                    }
                }
            }
            Ok(None) => {
                // Insert new record
                let result = sqlx::query(
                    "INSERT INTO balance_history (id, account_id, current_balance, total_consumed, total_income, recorded_at)
                     VALUES (?, ?, ?, ?, ?, ?)"
                )
                .bind(&id)
                .bind(account_id)
                .bind(balance.current_balance)
                .bind(balance.total_consumed)
                .bind(balance.total_income)
                .bind(now)
                .execute(&*self.pool)
                .await;

                match result {
                    Ok(_) => {
                        info!("Balance history saved for account {}: current=${:.2}, consumed=${:.2}, income=${:.2}",
                            account_id, balance.current_balance, balance.total_consumed, balance.total_income);
                    }
                    Err(e) => {
                        error!("Failed to save balance history: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to check balance history: {}", e);
            }
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
                    // Update account balance cache and save to balance_history if we have new balance data
                    let balance_dto = if result.success && result.user_info.is_some() {
                        let user_info = result.user_info.as_ref().unwrap();
                        let mut updated_account = account;
                        updated_account.update_balance(
                            user_info.quota,
                            user_info.used_quota,
                            user_info.quota + user_info.used_quota,
                        );

                        // Save updated account with new balance
                        if let Err(e) = self.account_repo.save(&updated_account).await {
                            error!("Failed to save account balance after check-in: {}", e);
                        } else {
                            info!("Account {} balance updated in database", account_id);
                        }

                        // Build balance DTO
                        let balance = BalanceDto {
                            current_balance: user_info.quota,
                            total_consumed: user_info.used_quota,
                            total_income: user_info.quota + user_info.used_quota,
                        };

                        // Save to balance_history table
                        self.save_balance_history(&account_id, &balance).await;

                        Some(balance)
                    } else {
                        result.user_info.as_ref().map(|info| BalanceDto {
                            current_balance: info.quota,
                            total_consumed: info.used_quota,
                            total_income: info.quota + info.used_quota,
                        })
                    };

                    // Send notification if service is available
                    if let Some(notification_service) = &self.notification_service {
                        let provider_name = provider.name();

                        if result.success {
                            let balance = result.user_info.as_ref().map(|info| {
                                (info.quota, info.used_quota, info.quota + info.used_quota)
                            });

                            if let Err(e) = notification_service
                                .send_check_in_success(&account_id, &result.account_name, provider_name, balance)
                                .await
                            {
                                error!("Failed to send check-in success notification: {}", e);
                            } else {
                                info!("Check-in success notification sent for account {}", account_id);
                            }
                        } else {
                            if let Err(e) = notification_service
                                .send_check_in_failure(&result.account_name, provider_name, &result.message)
                                .await
                            {
                                error!("Failed to send check-in failure notification: {}", e);
                            } else {
                                info!("Check-in failure notification sent for account {}", account_id);
                            }
                        }
                    } else {
                        info!("Notification service not available for account {}", account_id);
                    }

                    if result.success {
                        succeeded += 1;
                    } else {
                        failed += 1;
                    }
                    results.push(CheckInCommandResult {
                        success: result.success,
                        message: result.message,
                        balance: balance_dto,
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
