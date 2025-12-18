use anyhow::{Context, Result};
use log::{error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

use neuradock_domain::{
    account::{Account, AccountRepository},
    check_in::{CheckInDomainService, Provider},
    shared::AccountId,
};
use neuradock_infrastructure::http::{CheckInResult, HttpClient, UserInfo};
use neuradock_infrastructure::persistence::repositories::SqliteWafCookiesRepository;

use super::waf_cookie_manager::WafCookieManager;
use crate::application::config::TimeoutConfig;

/// Check-in result for a single account
#[derive(Debug, Clone)]
pub struct AccountCheckInResult {
    pub account_id: String,
    pub account_name: String,
    pub success: bool,
    pub message: String,
    pub user_info: Option<UserInfo>,
}

/// Batch check-in result
#[derive(Debug, Clone)]
pub struct BatchCheckInResult {
    pub total: usize,
    pub success_count: usize,
    pub failed_count: usize,
    pub results: Vec<AccountCheckInResult>,
}

/// Check-in executor service
pub struct CheckInExecutor {
    http_client: HttpClient,
    waf_manager: WafCookieManager,
    account_repo: Arc<dyn AccountRepository>,
    timeout_config: TimeoutConfig,
}

impl CheckInExecutor {
    pub fn new(account_repo: Arc<dyn AccountRepository>, headless_browser: bool) -> Result<Self> {
        let http_client = HttpClient::new()?;
        let waf_manager = WafCookieManager::new(headless_browser);

        Ok(Self {
            http_client,
            waf_manager,
            account_repo,
            timeout_config: TimeoutConfig::default(),
        })
    }

    /// Set WAF cookies repository for caching
    pub fn with_waf_cookies_repo(mut self, repo: Arc<SqliteWafCookiesRepository>) -> Self {
        self.waf_manager = self.waf_manager.with_cookies_repo(repo);
        self
    }

    /// Execute check-in for a single account
    #[instrument(skip(self, provider), fields(account_id = %account_id, provider_id = %provider.id()))]
    pub async fn execute_check_in(
        &self,
        account_id: &str,
        provider: &Provider,
    ) -> Result<AccountCheckInResult> {
        let account_id_obj = AccountId::from_string(account_id);

        // 1. Load and validate account
        let account = self.load_and_validate_account(&account_id_obj).await?;
        let account_name = account.name().to_string();

        info!("[{}] Starting check-in process", account_name);

        // 2. Validate using domain service
        if let Some(error_result) =
            self.validate_check_in_eligibility(&account, provider, account_id, &account_name)
        {
            return Ok(error_result);
        }

        // 3. Prepare cookies and fetch user info with WAF handling
        let (mut cookies, user_info) = self
            .prepare_cookies_and_fetch_user_info(&account, provider, &account_name)
            .await?;

        let api_user = account.credentials().api_user();

        // 4. Execute check-in if provider requires it
        let check_in_result = if let Some(sign_in_url) = provider.sign_in_url() {
            info!(
                "[{}] Executing check-in request to: {}",
                account_name, sign_in_url
            );

            // Check if the sign_in_url is a page (like /console/token) or an API endpoint
            // Page URLs typically don't contain /api/
            let is_page_visit = !sign_in_url.contains("/api/");

            if is_page_visit {
                // Visit the page (HTML response expected)
                info!("[{}] Visiting check-in page: {}", account_name, sign_in_url);
                match self
                    .http_client
                    .visit_login_page(&sign_in_url, &cookies)
                    .await
                {
                    Ok(_) => {
                        info!("[{}] Check-in page visited successfully!", account_name);
                        CheckInResult {
                            success: true,
                            message: "Check-in page visited successfully".to_string(),
                        }
                    }
                    Err(e) => {
                        error!("[{}] Failed to visit check-in page: {}", account_name, e);
                        CheckInResult {
                            success: false,
                            message: format!("Failed to visit page: {}", e),
                        }
                    }
                }
            } else {
                // Call API endpoint (JSON response expected)
                let check_in_call = self
                    .http_client
                    .execute_check_in(&sign_in_url, &cookies, provider.api_user_key(), api_user)
                    .await;

                match check_in_call {
                    Ok(result) => {
                        if result.success {
                            info!("[{}] Check-in successful!", account_name);
                        } else {
                            warn!("[{}] Check-in failed: {}", account_name, result.message);
                        }
                        result
                    }
                    Err(e) if self.waf_manager.is_waf_challenge_error(&e) => {
                        warn!("[{}] WAF challenge detected during check-in, refreshing cookies and retrying...", account_name);

                        // Refresh WAF cookies and retry
                        match self.waf_manager
                            .refresh_waf_cookies(
                                &account_name,
                                provider,
                                account.credentials().cookies(),
                            )
                            .await
                        {
                            Ok(fresh_cookies) => {
                                // Update main cookies variable for subsequent operations
                                cookies = fresh_cookies;

                                // Retry check-in with fresh cookies
                                match self
                                    .http_client
                                    .execute_check_in(
                                        &sign_in_url,
                                        &cookies,
                                        provider.api_user_key(),
                                        api_user,
                                    )
                                    .await
                                {
                                    Ok(result) => {
                                        info!(
                                            "[{}] Check-in retry successful after WAF refresh!",
                                            account_name
                                        );
                                        result
                                    }
                                    Err(retry_err) => {
                                        error!(
                                            "[{}] Check-in retry failed: {}",
                                            account_name, retry_err
                                        );
                                        CheckInResult {
                                            success: false,
                                            message: format!(
                                                "Check-in failed after WAF retry: {}",
                                                retry_err
                                            ),
                                        }
                                    }
                                }
                            }
                            Err(refresh_err) => {
                                error!(
                                    "[{}] Failed to refresh WAF cookies: {}",
                                    account_name, refresh_err
                                );
                                CheckInResult {
                                    success: false,
                                    message: format!("WAF refresh failed: {}", refresh_err),
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("[{}] Check-in request error: {}", account_name, e);
                        CheckInResult {
                            success: false,
                            message: format!("Request failed: {}", e),
                        }
                    }
                }
            }
        } else {
            info!(
                "[{}] Provider {} does not require an explicit check-in request, skipping API call",
                account_name,
                provider.name()
            );
            CheckInResult {
                success: true,
                message: "Provider does not require explicit check-in".to_string(),
            }
        };

        // Fetch balance after check-in to get updated value
        // For successful check-ins, always fetch updated balance
        let final_user_info = if check_in_result.success {
            info!(
                "[{}] Fetching updated balance after check-in...",
                account_name
            );

            // Wait for server to process check-in
            tokio::time::sleep(self.timeout_config.check_in_processing).await;

            match self
                .http_client
                .get_user_info(
                    &provider.user_info_url(),
                    &cookies,
                    provider.api_user_key(),
                    api_user,
                )
                .await
            {
                Ok(updated_info) => {
                    info!(
                        "[{}] Updated balance: ${:.2}, Used: ${:.2}",
                        account_name, updated_info.quota, updated_info.used_quota
                    );
                    Some(updated_info)
                }
                Err(e) => {
                    warn!(
                        "[{}] Failed to get updated balance: {}, using pre-check-in balance",
                        account_name, e
                    );
                    user_info
                }
            }
        } else {
            // Check-in failed, use initial balance if available
            user_info
        };

        Ok(AccountCheckInResult {
            account_id: account_id.to_string(),
            account_name,
            success: check_in_result.success,
            message: check_in_result.message,
            user_info: final_user_info,
        })
    }

    /// Fetch balance only (without triggering check-in)
    /// Only calls /api/user/self to get user info
    #[instrument(skip(self, provider), fields(account_id = %account_id, provider_id = %provider.id()))]
    pub async fn fetch_balance_only(
        &self,
        account_id: &str,
        provider: &Provider,
    ) -> Result<UserInfo> {
        let account_id_obj = AccountId::from_string(account_id);

        // Load account
        let account = self
            .account_repo
            .find_by_id(&account_id_obj)
            .await
            .context("Failed to load account")?
            .ok_or_else(|| anyhow::anyhow!("Account not found"))?;

        let account_name = account.name().to_string();

        info!(
            "[{}] Fetching balance (query only, no check-in)",
            account_name
        );

        // Prepare cookies
        let cookies = self.waf_manager
            .prepare_cookies(&account_name, provider, account.credentials().cookies())
            .await?;

        let api_user = account.credentials().api_user();

        // Get user info (balance)
        let user_info = self
            .http_client
            .get_user_info(
                &provider.user_info_url(),
                &cookies,
                provider.api_user_key(),
                api_user,
            )
            .await?;

        info!(
            "[{}] Balance fetched: ${:.2}, Used: ${:.2}",
            account_name, user_info.quota, user_info.used_quota
        );

        Ok(user_info)
    }

    // ========== Private helper methods for execute_check_in ==========

    /// Load and validate account exists
    async fn load_and_validate_account(&self, account_id: &AccountId) -> Result<Account> {
        self.account_repo
            .find_by_id(account_id)
            .await
            .context("Failed to load account")?
            .ok_or_else(|| anyhow::anyhow!("Account not found"))
    }

    /// Validate check-in eligibility using domain service
    fn validate_check_in_eligibility(
        &self,
        account: &Account,
        provider: &Provider,
        account_id: &str,
        account_name: &str,
    ) -> Option<AccountCheckInResult> {
        // Check account eligibility
        if let Err(e) = CheckInDomainService::can_check_in(account) {
            warn!("[{}] Check-in validation failed: {}", account_name, e);
            return Some(AccountCheckInResult {
                account_id: account_id.to_string(),
                account_name: account_name.to_string(),
                success: false,
                message: e.to_string(),
                user_info: None,
            });
        }

        // Validate provider configuration
        if let Err(e) = CheckInDomainService::validate_provider(provider) {
            error!("[{}] Provider validation failed: {}", account_name, e);
            return Some(AccountCheckInResult {
                account_id: account_id.to_string(),
                account_name: account_name.to_string(),
                success: false,
                message: e.to_string(),
                user_info: None,
            });
        }

        None
    }

    /// Prepare cookies and fetch user info with WAF handling
    async fn prepare_cookies_and_fetch_user_info(
        &self,
        account: &Account,
        provider: &Provider,
        account_name: &str,
    ) -> Result<(HashMap<String, String>, Option<UserInfo>)> {
        // Prepare cookies (with WAF cookies from cache or bypass)
        let mut cookies = self.waf_manager
            .prepare_cookies(account_name, provider, account.credentials().cookies())
            .await?;

        let api_user = account.credentials().api_user();

        // Get user info first
        let user_info_result = self
            .http_client
            .get_user_info(
                &provider.user_info_url(),
                &cookies,
                provider.api_user_key(),
                api_user,
            )
            .await;

        // Check if we got a WAF challenge and need to refresh cookies
        let user_info = match &user_info_result {
            Ok(info) => {
                info!(
                    "[{}] Current balance: ${:.2}, Used: ${:.2}",
                    account_name, info.quota, info.used_quota
                );
                Some(info.clone())
            }
            Err(e) if self.waf_manager.is_waf_challenge_error(e) => {
                warn!(
                    "[{}] WAF challenge detected, invalidating cache and retrying...",
                    account_name
                );

                // Invalidate WAF cache and get fresh cookies
                cookies = self.waf_manager
                    .refresh_waf_cookies(
                        account_name,
                        provider,
                        account.credentials().cookies(),
                    )
                    .await?;

                // Retry get user info
                match self
                    .http_client
                    .get_user_info(
                        &provider.user_info_url(),
                        &cookies,
                        provider.api_user_key(),
                        api_user,
                    )
                    .await
                {
                    Ok(info) => {
                        info!(
                            "[{}] Retry successful, balance: ${:.2}",
                            account_name, info.quota
                        );
                        Some(info)
                    }
                    Err(e) => {
                        warn!(
                            "[{}] Failed to get user info after retry: {}",
                            account_name, e
                        );
                        None
                    }
                }
            }
            Err(e) => {
                warn!("[{}] Failed to get user info: {}", account_name, e);
                None
            }
        };

        Ok((cookies, user_info))
    }

    /// Execute batch check-in for multiple accounts
    #[instrument(skip(self, providers), fields(batch_size = account_ids.len()))]
    pub async fn execute_batch_check_in(
        &self,
        account_ids: Vec<String>,
        providers: &HashMap<String, Provider>,
    ) -> Result<BatchCheckInResult> {
        let total = account_ids.len();
        let mut results = Vec::new();
        let mut success_count = 0;
        let mut failed_count = 0;

        // Batch load all accounts at once to avoid N+1 queries
        let account_id_objs: Vec<AccountId> = account_ids
            .iter()
            .map(|id| AccountId::from_string(id))
            .collect();
        
        let accounts = match self.account_repo.find_by_ids(&account_id_objs).await {
            Ok(accs) => accs,
            Err(e) => {
                error!("Failed to batch load accounts: {}", e);
                return Err(anyhow::anyhow!("Failed to load accounts: {}", e));
            }
        };

        // Create a map for quick lookup
        let account_map: HashMap<String, Account> = accounts
            .into_iter()
            .map(|acc| (acc.id().as_str().to_string(), acc))
            .collect();

        for account_id in account_ids {
            // Get account from pre-loaded map
            let account = match account_map.get(&account_id) {
                Some(acc) => acc,
                None => {
                    warn!("Account {} not found", account_id);
                    failed_count += 1;
                    continue;
                }
            };

            let provider_id = account.provider_id().as_str();
            let provider = match providers.get(provider_id) {
                Some(p) => p,
                None => {
                    warn!(
                        "Provider {} not found for account {}",
                        provider_id, account_id
                    );
                    failed_count += 1;
                    continue;
                }
            };

            // Execute check-in
            match self.execute_check_in(&account_id, provider).await {
                Ok(result) => {
                    if result.success {
                        success_count += 1;
                    } else {
                        failed_count += 1;
                    }
                    results.push(result);
                }
                Err(e) => {
                    error!("Check-in failed for account {}: {}", account_id, e);
                    failed_count += 1;
                    results.push(AccountCheckInResult {
                        account_id: account_id.clone(),
                        account_name: account.name().to_string(),
                        success: false,
                        message: format!("Error: {}", e),
                        user_info: None,
                    });
                }
            }
        }

        Ok(BatchCheckInResult {
            total,
            success_count,
            failed_count,
            results,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock tests would go here
    // Requires mockall for proper testing
}
