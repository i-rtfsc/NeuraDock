mod browser_setup;
mod cleanup;
mod navigation;
mod types;

use anyhow::Result;
use log::{info, warn};
use std::collections::HashMap;
use std::time::Duration;

use browser_setup::find_browser;
use cleanup::cleanup_browser;
use types::REQUIRED_WAF_COOKIES;

pub struct WafBypassService {
    headless: bool,
}

impl WafBypassService {
    pub fn new(headless: bool) -> Self {
        Self { headless }
    }

    /// Get WAF cookies using chromiumoxide (pure Rust)
    pub async fn get_waf_cookies(
        &self,
        login_url: &str,
        account_name: &str,
    ) -> Result<HashMap<String, String>> {
        const MAX_RETRIES: u32 = 2;
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                info!(
                    "[{}] Retrying WAF cookie fetch (attempt {}/{})",
                    account_name,
                    attempt + 1,
                    MAX_RETRIES
                );
                tokio::time::sleep(Duration::from_secs(2)).await;
            }

            match self.get_waf_cookies_once(login_url, account_name).await {
                Ok(cookies) => return Ok(cookies),
                Err(e) => {
                    warn!(
                        "[{}] WAF cookie fetch attempt {} failed: {}",
                        account_name,
                        attempt + 1,
                        e
                    );
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!("Failed to get WAF cookies after {} attempts", MAX_RETRIES)
        }))
    }

    /// Internal method to get WAF cookies once
    async fn get_waf_cookies_once(
        &self,
        login_url: &str,
        account_name: &str,
    ) -> Result<HashMap<String, String>> {
        info!(
            "[{}] Starting browser to get WAF cookies (chromiumoxide)...",
            account_name
        );

        // 1. Launch browser with proper configuration
        let (browser, handler_task, temp_dir) = self
            .launch_browser_with_config(account_name)
            .await?;

        // 2. Navigate to page and extract cookies
        let (browser, waf_cookies_result) = self
            .navigate_and_extract_cookies(
                browser,
                login_url,
                account_name,
            )
            .await;

        // 3. Clean up browser resources (always execute even if error)
        cleanup_browser(browser, handler_task, temp_dir, account_name).await;

        // 4. Return result
        let waf_cookies = waf_cookies_result?;

        // Check if we got any cookies
        if waf_cookies.is_empty() {
            let err_msg = format!(
                "No WAF cookies obtained. Expected cookies: {:?}. This might indicate that the page didn't load properly or WAF protection has changed.",
                REQUIRED_WAF_COOKIES
            );
            warn!("[{}] {}", account_name, err_msg);
            anyhow::bail!(err_msg);
        }

        info!(
            "[{}] ✓ Successfully got {} WAF cookies",
            account_name,
            waf_cookies.len()
        );

        Ok(waf_cookies)
    }
}

impl Default for WafBypassService {
    fn default() -> Self {
        Self::new(true) // Headless by default
    }
}

/// Check which browser is available on the system
pub fn check_available_browser() -> Option<String> {
    find_browser().map(|path| path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waf_service_creation() {
        let service = WafBypassService::new(true);
        assert!(service.headless);
    }

    #[test]
    fn test_browser_detection() {
        let browser = find_browser();
        println!("Found browser: {:?}", browser);
        // This test will pass even if no browser is found
        // It's just for checking during development
    }
}
