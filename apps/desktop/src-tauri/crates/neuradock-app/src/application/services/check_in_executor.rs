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
}

impl CheckInExecutor {
    pub fn new(account_repo: Arc<dyn AccountRepository>, headless_browser: bool) -> Result<Self> {
        let http_client = HttpClient::new()?;
        let waf_service = WafBypassService::new(headless_browser);

        Ok(Self {
            http_client,
            waf_service,
            account_repo,
        })
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

        // Prepare cookies
        let cookies = self
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

        let user_info = match user_info_result {
            Ok(info) => {
                info!(
                    "[{}] Current balance: ${:.2}, Used: ${:.2}",
                    account_name, info.quota, info.used_quota
                );
                Some(info)
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
                match self
                    .http_client
                    .execute_check_in(&sign_in_url, &cookies, provider.api_user_key(), api_user)
                    .await
                {
                    Ok(result) => {
                        if result.success {
                            info!("[{}] Check-in successful!", account_name);
                        } else {
                            warn!("[{}] Check-in failed: {}", account_name, result.message);
                        }
                        result
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
            for endpoint in &api_endpoints {
                match self
                    .http_client
                    .call_api_endpoint(endpoint, &cookies, provider.api_user_key(), api_user)
                    .await
                {
                    Ok(_) => {
                        info!(
                            "[{}] API endpoint called successfully: {}",
                            account_name, endpoint
                        );
                        success_count += 1;
                    }
                    Err(e) => {
                        warn!(
                            "[{}] Failed to call API endpoint {} (non-critical): {}",
                            account_name, endpoint, e
                        );
                    }
                }
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

        // For auto check-in providers (no sign_in_url), fetch balance again to get updated value
        // Always retry for auto check-in providers, even if first call failed
        let final_user_info = if provider.sign_in_url().is_none() {
            info!(
                "[{}] Fetching updated balance after auto check-in...",
                account_name
            );

            // Wait longer for server to process check-in (increased from 500ms to 2s)
            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

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
                        "[{}] Failed to get updated balance: {}, fallback to initial balance",
                        account_name, e
                    );
                    user_info
                }
            }
        } else {
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

    /// Prepare cookies with WAF bypass if needed
    async fn prepare_cookies(
        &self,
        account_name: &str,
        provider: &Provider,
        user_cookies: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>> {
        let mut cookies = user_cookies.clone();

        if provider.needs_waf_bypass() {
            info!(
                "[{}] WAF bypass required, getting WAF cookies via Python script...",
                account_name
            );

            let waf_cookies = self
                .waf_service
                .get_waf_cookies(&provider.login_url(), account_name)
                .await
                .context("Failed to get WAF cookies")?;

            // Merge WAF cookies with user cookies (WAF cookies first)
            cookies.extend(waf_cookies);
        } else {
            info!("[{}] No WAF bypass required", account_name);
        }

        Ok(cookies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock tests would go here
    // Requires mockall for proper testing
}
