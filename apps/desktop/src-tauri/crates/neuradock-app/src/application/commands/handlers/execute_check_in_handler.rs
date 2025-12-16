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
use neuradock_domain::check_in::{Provider, ProviderRepository};
use neuradock_domain::shared::{AccountId, DomainError};
use neuradock_infrastructure::http::token::TokenClient;
use neuradock_infrastructure::persistence::repositories::{
    SqliteProviderModelsRepository, SqliteWafCookiesRepository,
};

/// Execute check-in command handler
pub struct ExecuteCheckInCommandHandler {
    account_repo: Arc<dyn AccountRepository>,
    provider_repo: Arc<dyn ProviderRepository>,
    notification_service: Option<Arc<NotificationService>>,
    provider_models_repo: Arc<SqliteProviderModelsRepository>,
    waf_cookies_repo: Arc<SqliteWafCookiesRepository>,
    headless_browser: bool,
    pool: Arc<SqlitePool>,
}

impl ExecuteCheckInCommandHandler {
    pub fn new(
        account_repo: Arc<dyn AccountRepository>,
        provider_repo: Arc<dyn ProviderRepository>,
        provider_models_repo: Arc<SqliteProviderModelsRepository>,
        waf_cookies_repo: Arc<SqliteWafCookiesRepository>,
        headless_browser: bool,
        pool: Arc<SqlitePool>,
    ) -> Self {
        Self {
            account_repo,
            provider_repo,
            notification_service: None,
            provider_models_repo,
            waf_cookies_repo,
            headless_browser,
            pool,
        }
    }

    pub fn with_notification_service(mut self, service: Arc<NotificationService>) -> Self {
        self.notification_service = Some(service);
        self
    }

    /// Fetch and save provider models after successful check-in
    async fn fetch_and_save_provider_models(
        &self,
        provider: &Provider,
        cookies: &HashMap<String, String>,
        api_user: &str,
    ) {
        // Check if provider has models API
        let models_path = match provider.models_path() {
            Some(path) => path,
            None => {
                info!(
                    "Provider {} does not have models API, skipping",
                    provider.name()
                );
                return;
            }
        };
        let provider_id = provider.id().as_str();
        let base_url = provider.domain().trim_end_matches('/').to_string();
        let api_user_header = provider.api_user_key();
        let api_user_header_opt = if api_user_header.is_empty() {
            None
        } else {
            Some(api_user_header)
        };

        // Build cookies with WAF cookies from cache
        let mut all_cookies = cookies.clone();

        // Try to get cached WAF cookies
        match self.waf_cookies_repo.get_valid(provider_id).await {
            Ok(Some(cached_waf)) => {
                info!("Using cached WAF cookies for provider models fetch");
                all_cookies.extend(cached_waf.cookies);
            }
            Ok(None) => {
                info!("No cached WAF cookies available for provider models fetch");
            }
            Err(e) => {
                error!("Failed to get cached WAF cookies: {}", e);
            }
        }

        // Build cookie string
        let cookie_string: String = all_cookies
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ");

        // Create token client and fetch models
        let client = match TokenClient::new() {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to create token client: {}", e);
                return;
            }
        };

        // Use account's api_user value, not provider's api_user_key (which is the header name)
        let api_user_opt = if api_user.is_empty() {
            None
        } else {
            Some(api_user)
        };

        match client
            .fetch_provider_models(
                &base_url,
                models_path,
                &cookie_string,
                api_user_header_opt,
                api_user_opt,
            )
            .await
        {
            Ok(models) => {
                info!(
                    "Fetched {} models for provider {}",
                    models.len(),
                    provider.name()
                );

                // Save to database
                if let Err(e) = self.provider_models_repo.save(provider_id, &models).await {
                    error!("Failed to save provider models: {}", e);
                } else {
                    info!("Provider models saved to database");
                }
            }
            Err(e) => {
                // Don't fail the check-in if models fetch fails
                error!("Failed to fetch provider models (non-critical): {}", e);
            }
        }
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
                     WHERE id = ?",
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

        // Create executor
        let executor = CheckInExecutor::new(self.account_repo.clone(), self.headless_browser)
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?
            .with_waf_cookies_repo(self.waf_cookies_repo.clone());

        // Execute check-in
        let result = executor
            .execute_check_in(&cmd.account_id, &provider)
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

            // Auto-fetch provider models if not exists in database
            // Check if provider models already exist
            if let Ok(existing_models) = self
                .provider_models_repo
                .find_by_provider(provider.id().as_str())
                .await
            {
                if existing_models.is_none()
                    || existing_models
                        .as_ref()
                        .map(|m| m.models.is_empty())
                        .unwrap_or(true)
                {
                    info!("No provider models in database, auto-fetching...");
                    // Reload account to get the latest cookies (may have been updated during check-in)
                    if let Ok(Some(updated_acc)) = self
                        .account_repo
                        .find_by_id(&AccountId::from_string(&cmd.account_id))
                        .await
                    {
                        self.fetch_and_save_provider_models(
                            &provider,
                            updated_acc.credentials().cookies(),
                            updated_acc.credentials().api_user(),
                        )
                        .await;
                    }
                } else {
                    info!("Provider models already exist in database, skipping auto-fetch");
                }
            }

            Some(balance)
        } else {
            None
        };

        // Send notification if service is available
        if let Some(notification_service) = &self.notification_service {
            let provider_name = provider.name();

            if result.success {
                // Send success notification
                let balance = result
                    .user_info
                    .as_ref()
                    .map(|info| (info.quota, info.used_quota, info.quota + info.used_quota));

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
    provider_repo: Arc<dyn ProviderRepository>,
    notification_service: Option<Arc<NotificationService>>,
    provider_models_repo: Arc<SqliteProviderModelsRepository>,
    waf_cookies_repo: Arc<SqliteWafCookiesRepository>,
    headless_browser: bool,
    pool: Arc<SqlitePool>,
}

impl BatchExecuteCheckInCommandHandler {
    pub fn new(
        account_repo: Arc<dyn AccountRepository>,
        provider_repo: Arc<dyn ProviderRepository>,
        provider_models_repo: Arc<SqliteProviderModelsRepository>,
        waf_cookies_repo: Arc<SqliteWafCookiesRepository>,
        headless_browser: bool,
        pool: Arc<SqlitePool>,
    ) -> Self {
        Self {
            account_repo,
            provider_repo,
            notification_service: None,
            provider_models_repo,
            waf_cookies_repo,
            headless_browser,
            pool,
        }
    }

    pub fn with_notification_service(mut self, service: Arc<NotificationService>) -> Self {
        self.notification_service = Some(service);
        self
    }

    /// Fetch and save provider models after successful check-in
    async fn fetch_and_save_provider_models(
        &self,
        provider: &Provider,
        cookies: &HashMap<String, String>,
        api_user: &str,
    ) {
        // Check if provider has models API
        let models_path = match provider.models_path() {
            Some(path) => path,
            None => {
                info!(
                    "Provider {} does not have models API, skipping",
                    provider.name()
                );
                return;
            }
        };
        let provider_id = provider.id().as_str();
        let base_url = provider.domain().trim_end_matches('/').to_string();
        let api_user_header = provider.api_user_key();
        let api_user_header_opt = if api_user_header.is_empty() {
            None
        } else {
            Some(api_user_header)
        };

        // Build cookies with WAF cookies from cache
        let mut all_cookies = cookies.clone();

        // Try to get cached WAF cookies
        match self.waf_cookies_repo.get_valid(provider_id).await {
            Ok(Some(cached_waf)) => {
                info!("Using cached WAF cookies for provider models fetch");
                all_cookies.extend(cached_waf.cookies);
            }
            Ok(None) => {
                info!("No cached WAF cookies available for provider models fetch");
            }
            Err(e) => {
                error!("Failed to get cached WAF cookies: {}", e);
            }
        }

        // Build cookie string
        let cookie_string: String = all_cookies
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ");

        // Create token client and fetch models
        let client = match TokenClient::new() {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to create token client: {}", e);
                return;
            }
        };

        // Use account's api_user value, not provider's api_user_key (which is the header name)
        let api_user_opt = if api_user.is_empty() {
            None
        } else {
            Some(api_user)
        };

        match client
            .fetch_provider_models(
                &base_url,
                models_path,
                &cookie_string,
                api_user_header_opt,
                api_user_opt,
            )
            .await
        {
            Ok(models) => {
                info!(
                    "Fetched {} models for provider {}",
                    models.len(),
                    provider.name()
                );

                // Save to database
                if let Err(e) = self.provider_models_repo.save(provider_id, &models).await {
                    error!("Failed to save provider models: {}", e);
                } else {
                    info!("Provider models saved to database");
                }
            }
            Err(e) => {
                // Don't fail the check-in if models fetch fails
                error!("Failed to fetch provider models (non-critical): {}", e);
            }
        }
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
                     WHERE id = ?",
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
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?
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
            let provider_id = account.provider_id().as_str().to_string();
            let provider = match self.provider_repo.find_by_id(account.provider_id()).await {
                Ok(Some(provider)) => provider,
                Ok(None) => {
                    error!("Provider not found: {}", provider_id);
                    failed += 1;
                    results.push(CheckInCommandResult {
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

                        // Auto-fetch provider models if not exists in database
                        // Check if provider models already exist
                        if let Ok(existing_models) = self
                            .provider_models_repo
                            .find_by_provider(provider.id().as_str())
                            .await
                        {
                            if existing_models.is_none()
                                || existing_models
                                    .as_ref()
                                    .map(|m| m.models.is_empty())
                                    .unwrap_or(true)
                            {
                                info!("No provider models in database, auto-fetching...");
                                // Reload account to get the latest cookies
                                if let Ok(Some(updated_acc)) = self
                                    .account_repo
                                    .find_by_id(&AccountId::from_string(&account_id))
                                    .await
                                {
                                    self.fetch_and_save_provider_models(
                                        &provider,
                                        updated_acc.credentials().cookies(),
                                        updated_acc.credentials().api_user(),
                                    )
                                    .await;
                                }
                            } else {
                                info!("Provider models already exist in database, skipping auto-fetch");
                            }
                        }

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
                                .send_check_in_success(
                                    &account_id,
                                    &result.account_name,
                                    provider_name,
                                    balance,
                                )
                                .await
                            {
                                error!("Failed to send check-in success notification: {}", e);
                            } else {
                                info!(
                                    "Check-in success notification sent for account {}",
                                    account_id
                                );
                            }
                        } else {
                            if let Err(e) = notification_service
                                .send_check_in_failure(
                                    &result.account_name,
                                    provider_name,
                                    &result.message,
                                )
                                .await
                            {
                                error!("Failed to send check-in failure notification: {}", e);
                            } else {
                                info!(
                                    "Check-in failure notification sent for account {}",
                                    account_id
                                );
                            }
                        }
                    } else {
                        info!(
                            "Notification service not available for account {}",
                            account_id
                        );
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
