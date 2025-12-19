use anyhow::Result;
use log::{info, warn};
use neuradock_domain::check_in::Provider;
use neuradock_infrastructure::http::{HttpClient, UserInfo};
use std::collections::HashMap;

use super::waf_cookie_manager::WafCookieManager;
use crate::application::config::TimeoutConfig;

/// Service for fetching user info and balance with WAF retry handling
pub struct UserInfoService<'a> {
    http_client: &'a HttpClient,
    waf_manager: &'a WafCookieManager,
    timeout_config: TimeoutConfig,
}

impl<'a> UserInfoService<'a> {
    pub fn new(http_client: &'a HttpClient, waf_manager: &'a WafCookieManager) -> Self {
        Self {
            http_client,
            waf_manager,
            timeout_config: TimeoutConfig::default(),
        }
    }

    /// Fetch user info with automatic WAF retry handling
    /// Returns (cookies, user_info) where cookies may be updated after WAF refresh
    pub async fn fetch_user_info_with_retry(
        &self,
        account_name: &str,
        provider: &Provider,
        account_cookies: &HashMap<String, String>,
        api_user: &str,
    ) -> Result<(HashMap<String, String>, Option<UserInfo>)> {
        // Prepare cookies (with WAF cookies from cache or bypass)
        let mut cookies = self
            .waf_manager
            .prepare_cookies(account_name, provider, account_cookies)
            .await?;

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
                cookies = self
                    .waf_manager
                    .refresh_waf_cookies(account_name, provider, account_cookies)
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

    /// Fetch updated balance after successful check-in
    /// Waits for server to process check-in before fetching
    pub async fn fetch_updated_balance(
        &self,
        account_name: &str,
        provider: &Provider,
        cookies: &HashMap<String, String>,
        api_user: &str,
        initial_user_info: Option<UserInfo>,
    ) -> Option<UserInfo> {
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
                cookies,
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
                initial_user_info
            }
        }
    }

    /// Fetch user info without WAF retry (simpler version for query-only operations)
    pub async fn fetch_user_info(
        &self,
        account_name: &str,
        provider: &Provider,
        cookies: &HashMap<String, String>,
        api_user: &str,
    ) -> Result<UserInfo> {
        let user_info = self
            .http_client
            .get_user_info(
                &provider.user_info_url(),
                cookies,
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
}
