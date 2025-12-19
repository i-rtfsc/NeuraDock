use anyhow::{Context, Result};
use neuradock_domain::account::Account;
use neuradock_domain::check_in::Provider;
use std::collections::HashMap;

impl super::TokenService {
    /// Get fresh WAF cookies via browser bypass (skips cache)
    pub(super) async fn get_fresh_waf_cookies(
        &self,
        provider: &Provider,
        account: &Account,
    ) -> Result<HashMap<String, String>> {
        let login_url = provider.login_url();

        let waf_cookies = self
            .waf_service
            .get_waf_cookies(&login_url, account.name())
            .await
            .context("Failed to get WAF cookies")?;

        log::info!(
            "Successfully got {} WAF cookies via bypass",
            waf_cookies.len()
        );

        // Cache the new WAF cookies
        if let Some(ref waf_cookies_repo) = self.waf_cookies_repo {
            if let Err(e) = waf_cookies_repo
                .save(provider.id().as_str(), &waf_cookies)
                .await
            {
                log::warn!("Failed to cache WAF cookies: {}", e);
            } else {
                log::info!("WAF cookies cached for 24 hours");
            }
        }

        Ok(waf_cookies)
    }
}
