use anyhow::{Context, Result};
use log::{info, warn};
use std::collections::HashMap;
use std::sync::Arc;

use neuradock_domain::check_in::Provider;
use neuradock_infrastructure::http::WafBypassService;
use neuradock_infrastructure::persistence::repositories::SqliteWafCookiesRepository;

/// Service for managing WAF cookies with caching support
pub struct WafCookieManager {
    waf_service: WafBypassService,
    waf_cookies_repo: Option<Arc<SqliteWafCookiesRepository>>,
}

impl WafCookieManager {
    /// Create a new WAF cookie manager
    pub fn new(headless_browser: bool) -> Self {
        Self {
            waf_service: WafBypassService::new(headless_browser),
            waf_cookies_repo: None,
        }
    }

    /// Set WAF cookies repository for caching
    pub fn with_cookies_repo(mut self, repo: Arc<SqliteWafCookiesRepository>) -> Self {
        self.waf_cookies_repo = Some(repo);
        self
    }

    /// Prepare cookies with WAF bypass if needed (with caching support)
    pub async fn prepare_cookies(
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
                        warn!(
                            "[{}] Failed to check cached WAF cookies: {}",
                            account_name, e
                        );
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
            self.cache_waf_cookies(account_name, provider_id, &waf_cookies)
                .await;

            // Merge WAF cookies with user cookies
            cookies.extend(waf_cookies);
        } else {
            info!("[{}] No WAF bypass required", account_name);
        }

        Ok(cookies)
    }

    /// Check if an error indicates a WAF challenge response
    pub fn is_waf_challenge_error(&self, error: &anyhow::Error) -> bool {
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
    pub async fn refresh_waf_cookies(
        &self,
        account_name: &str,
        provider: &Provider,
        user_cookies: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>> {
        let provider_id = provider.id().as_str();

        // Delete cached WAF cookies
        self.invalidate_cache(account_name, provider_id).await;

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
        self.cache_waf_cookies(account_name, provider_id, &waf_cookies)
            .await;

        // Merge with user cookies
        let mut cookies = user_cookies.clone();
        cookies.extend(waf_cookies);

        Ok(cookies)
    }

    /// Cache WAF cookies for future use
    async fn cache_waf_cookies(
        &self,
        account_name: &str,
        provider_id: &str,
        waf_cookies: &HashMap<String, String>,
    ) {
        if let Some(ref waf_cookies_repo) = self.waf_cookies_repo {
            if let Err(e) = waf_cookies_repo.save(provider_id, waf_cookies).await {
                warn!("[{}] Failed to cache WAF cookies: {}", account_name, e);
            } else {
                info!("[{}] WAF cookies cached for 24 hours", account_name);
            }
        }
    }

    /// Invalidate cached WAF cookies for a provider
    async fn invalidate_cache(&self, account_name: &str, provider_id: &str) {
        if let Some(ref waf_cookies_repo) = self.waf_cookies_repo {
            if let Err(e) = waf_cookies_repo.delete(provider_id).await {
                warn!(
                    "[{}] Failed to delete cached WAF cookies: {}",
                    account_name, e
                );
            } else {
                info!("[{}] Invalidated cached WAF cookies", account_name);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_waf_challenge_error() {
        let manager = WafCookieManager::new(true);

        // Test WAF_CHALLENGE marker
        let error = anyhow::anyhow!("WAF_CHALLENGE detected");
        assert!(manager.is_waf_challenge_error(&error));

        // Test acw_sc__v2 cookie
        let error = anyhow::anyhow!("Missing acw_sc__v2 cookie");
        assert!(manager.is_waf_challenge_error(&error));

        // Test Cloudflare challenge
        let error = anyhow::anyhow!("just a moment...");
        assert!(manager.is_waf_challenge_error(&error));

        // Test normal error
        let error = anyhow::anyhow!("Network timeout");
        assert!(!manager.is_waf_challenge_error(&error));
    }
}
