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
use neuradock_infrastructure::http::{CheckInResult, HttpClient, UserInfo, WafBypassService};
use neuradock_infrastructure::persistence::repositories::SqliteWafCookiesRepository;

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
    waf_service: WafBypassService,
    account_repo: Arc<dyn AccountRepository>,
    waf_cookies_repo: Option<Arc<SqliteWafCookiesRepository>>,
}

impl CheckInExecutor {
    pub fn new(account_repo: Arc<dyn AccountRepository>, headless_browser: bool) -> Result<Self> {
        let http_client = HttpClient::new()?;
        let waf_service = WafBypassService::new(headless_browser);

        Ok(Self {
            http_client,
            waf_service,
            account_repo,
            waf_cookies_repo: None,
        })
    }

    /// Set WAF cookies repository for caching
    pub fn with_waf_cookies_repo(mut self, repo: Arc<SqliteWafCookiesRepository>) -> Self {
        self.waf_cookies_repo = Some(repo);
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

        // Load account
        let account = self
            .account_repo
            .find_by_id(&account_id_obj)
            .await
            .context("Failed to load account")?
            .ok_or_else(|| anyhow::anyhow!("Account not found"))?;

        let account_name = account.name().to_string();

        info!("[{}] Starting check-in process", account_name);

        // Use domain service to validate check-in eligibility
        if let Err(e) = CheckInDomainService::can_check_in(&account) {
            warn!("[{}] Check-in validation failed: {}", account_name, e);
            return Ok(AccountCheckInResult {
                account_id: account_id.to_string(),
                account_name,
                success: false,
                message: e.to_string(),
                user_info: None,
            });
        }

        // Validate provider configuration
        if let Err(e) = CheckInDomainService::validate_provider(provider) {
            error!("[{}] Provider validation failed: {}", account_name, e);
            return Ok(AccountCheckInResult {
                account_id: account_id.to_string(),
                account_name,
                success: false,
                message: e.to_string(),
                user_info: None,
            });
        }

        // Prepare cookies (with WAF cookies from cache or bypass)
        let mut cookies = self
            .prepare_cookies(&account_name, provider, account.credentials().cookies())
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
            Err(e) if self.is_waf_challenge_error(e) => {
                warn!("[{}] WAF challenge detected, invalidating cache and retrying...", account_name);

                // Invalidate WAF cache and get fresh cookies
                cookies = self
                    .refresh_waf_cookies_and_retry(&account_name, provider, account.credentials().cookies())
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
                        info!("[{}] Retry successful, balance: ${:.2}", account_name, info.quota);
                        Some(info)
                    }
                    Err(e) => {
                        warn!("[{}] Failed to get user info after retry: {}", account_name, e);
                        None
                    }
                }
            }
            Err(e) => {
                warn!("[{}] Failed to get user info: {}", account_name, e);
                None
            }
        };

        // Execute check-in if provider requires it
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
                    Err(e) if self.is_waf_challenge_error(&e) => {
                        warn!("[{}] WAF challenge detected during check-in, refreshing cookies and retrying...", account_name);

                        // Refresh WAF cookies and retry
                        match self
                            .refresh_waf_cookies_and_retry(&account_name, provider, account.credentials().cookies())
                            .await
                        {
                            Ok(fresh_cookies) => {
                                // Update main cookies variable for subsequent operations
                                cookies = fresh_cookies;

                                // Retry check-in with fresh cookies
                                match self
                                    .http_client
                                    .execute_check_in(&sign_in_url, &cookies, provider.api_user_key(), api_user)
                                    .await
                                {
                                    Ok(result) => {
                                        info!("[{}] Check-in retry successful after WAF refresh!", account_name);
                                        result
                                    }
                                    Err(retry_err) => {
                                        error!("[{}] Check-in retry failed: {}", account_name, retry_err);
                                        CheckInResult {
                                            success: false,
                                            message: format!("Check-in failed after WAF retry: {}", retry_err),
                                        }
                                    }
                                }
                            }
                            Err(refresh_err) => {
                                error!("[{}] Failed to refresh WAF cookies: {}", account_name, refresh_err);
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
            // No sign_in_url specified - for AgentRouter-like providers, call API endpoints to trigger balance update
            info!(
                "[{}] Calling API endpoints to trigger balance update...",
                account_name
            );

            // For AgentRouter, call all 4 API endpoints that are triggered during re-login
            let api_endpoints = vec![
                format!("{}/api/status", provider.domain()),
                format!("{}/api/user/models", provider.domain()),
                format!("{}/api/user/self/groups", provider.domain()),
                format!("{}/api/token/?p=1&size=10", provider.domain()),
            ];

            let mut success_count = 0;
            let mut waf_refreshed = false;
            let mut current_cookies = cookies.clone();

            for endpoint in &api_endpoints {
                let result = self
                    .http_client
                    .call_api_endpoint(endpoint, &current_cookies, provider.api_user_key(), api_user)
                    .await;

                match result {
                    Ok(_) => {
                        info!(
                            "[{}] API endpoint called successfully: {}",
                            account_name, endpoint
                        );
                        success_count += 1;
                    }
                    Err(e) if self.is_waf_challenge_error(&e) && !waf_refreshed => {
                        warn!(
                            "[{}] WAF challenge during API endpoint {}, refreshing cookies...",
                            account_name, endpoint
                        );

                        // Refresh WAF cookies and retry
                        if let Ok(fresh_cookies) = self
                            .refresh_waf_cookies_and_retry(&account_name, provider, account.credentials().cookies())
                            .await
                        {
                            current_cookies = fresh_cookies;
                            waf_refreshed = true;

                            // Retry this endpoint
                            if self
                                .http_client
                                .call_api_endpoint(endpoint, &current_cookies, provider.api_user_key(), api_user)
                                .await
                                .is_ok()
                            {
                                info!(
                                    "[{}] API endpoint retry successful: {}",
                                    account_name, endpoint
                                );
                                success_count += 1;
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            "[{}] Failed to call API endpoint {} (non-critical): {}",
                            account_name, endpoint, e
                        );
                    }
                }
            }

            // Update cookies for subsequent operations if WAF was refreshed
            if waf_refreshed {
                cookies = current_cookies;
            }

            info!(
                "[{}] Check-in completed ({}/{} API endpoints called successfully)",
                account_name,
                success_count,
                api_endpoints.len()
            );
            CheckInResult {
                success: true,
                message: format!(
                    "Check-in completed ({}/{} API calls)",
                    success_count,
                    api_endpoints.len()
                ),
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
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

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

        info!("[{}] Fetching balance (query only, no check-in)", account_name);

        // Prepare cookies
        let cookies = self
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

        for account_id in account_ids {
            // Load account to get provider_id
            let account_id_obj = AccountId::from_string(&account_id);
            let account = match self.account_repo.find_by_id(&account_id_obj).await {
                Ok(Some(acc)) => acc,
                Ok(None) => {
                    warn!("Account {} not found", account_id);
                    failed_count += 1;
                    continue;
                }
                Err(e) => {
                    error!("Failed to load account {}: {}", account_id, e);
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

    /// Prepare cookies with WAF bypass if needed (with caching support)
    async fn prepare_cookies(
        &self,
        account_name: &str,
        provider: &Provider,
        user_cookies: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>> {
        let mut cookies = user_cookies.clone();

        if provider.needs_waf_bypass() {
            let provider_id = provider.id().as_str();

            // Try to use cached WAF cookies first
            if let Some(ref waf_cookies_repo) = self.waf_cookies_repo {
                match waf_cookies_repo.get_valid(provider_id).await {
                    Ok(Some(cached_waf)) => {
                        info!(
                            "[{}] Using cached WAF cookies (expires at {})",
                            account_name, cached_waf.expires_at
                        );
                        cookies.extend(cached_waf.cookies);
                        return Ok(cookies);
                    }
                    Ok(None) => {
                        info!("[{}] No valid cached WAF cookies found", account_name);
                    }
                    Err(e) => {
                        warn!("[{}] Failed to check cached WAF cookies: {}", account_name, e);
                    }
                }
            }

            // No valid cache, run WAF bypass
            info!(
                "[{}] WAF bypass required, getting WAF cookies via browser...",
                account_name
            );

            let waf_cookies = self
                .waf_service
                .get_waf_cookies(&provider.login_url(), account_name)
                .await
                .context("Failed to get WAF cookies")?;

            // Cache the new WAF cookies
            if let Some(ref waf_cookies_repo) = self.waf_cookies_repo {
                if let Err(e) = waf_cookies_repo.save(provider_id, &waf_cookies).await {
                    warn!("[{}] Failed to cache WAF cookies: {}", account_name, e);
                } else {
                    info!("[{}] WAF cookies cached for 24 hours", account_name);
                }
            }

            // Merge WAF cookies with user cookies
            cookies.extend(waf_cookies);
        } else {
            info!("[{}] No WAF bypass required", account_name);
        }

        Ok(cookies)
    }

    /// Check if an error indicates a WAF challenge response
    fn is_waf_challenge_error(&self, error: &anyhow::Error) -> bool {
        let error_str = error.to_string();
        let error_lower = error_str.to_lowercase();
        // WAF challenge indicators:
        // - WAF_CHALLENGE marker from HTTP client
        // - JavaScript challenge page with acw_sc__v2 cookie generation
        // - Cloudflare challenge page
        // - 403 with challenge content
        error_str.contains("WAF_CHALLENGE")
            || error_lower.contains("acw_sc__v2")
            || error_lower.contains("waf")
            || error_str.contains("<script>var arg1=")
            || error_lower.contains("just a moment")
            || error_lower.contains("checking your browser")
    }

    /// Invalidate WAF cache and get fresh cookies
    async fn refresh_waf_cookies_and_retry(
        &self,
        account_name: &str,
        provider: &Provider,
        user_cookies: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>> {
        let provider_id = provider.id().as_str();

        // Delete cached WAF cookies
        if let Some(ref waf_cookies_repo) = self.waf_cookies_repo {
            if let Err(e) = waf_cookies_repo.delete(provider_id).await {
                warn!("[{}] Failed to delete cached WAF cookies: {}", account_name, e);
            } else {
                info!("[{}] Invalidated cached WAF cookies", account_name);
            }
        }

        // Run fresh WAF bypass
        info!(
            "[{}] Running fresh WAF bypass after challenge detection...",
            account_name
        );

        let waf_cookies = self
            .waf_service
            .get_waf_cookies(&provider.login_url(), account_name)
            .await
            .context("Failed to get fresh WAF cookies after challenge")?;

        info!(
            "[{}] Got {} fresh WAF cookies",
            account_name,
            waf_cookies.len()
        );

        // Cache the new WAF cookies
        if let Some(ref waf_cookies_repo) = self.waf_cookies_repo {
            if let Err(e) = waf_cookies_repo.save(provider_id, &waf_cookies).await {
                warn!("[{}] Failed to cache new WAF cookies: {}", account_name, e);
            } else {
                info!("[{}] New WAF cookies cached for 24 hours", account_name);
            }
        }

        // Merge with user cookies
        let mut cookies = user_cookies.clone();
        cookies.extend(waf_cookies);

        Ok(cookies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock tests would go here
    // Requires mockall for proper testing
}
