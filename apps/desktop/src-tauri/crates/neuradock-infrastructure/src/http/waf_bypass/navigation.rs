use anyhow::Result;
use chromiumoxide::browser::Browser;
use log::info;
use std::collections::HashMap;

use super::types::{REQUIRED_WAF_COOKIES, USER_AGENT};
use crate::config::TimeoutConfig;
use crate::logging::log_utils::mask_sensitive;

impl super::WafBypassService {
    /// Navigate to page and extract WAF cookies
    /// Returns (browser, cookies_result) to allow cleanup even on error
    pub(super) async fn navigate_and_extract_cookies(
        &self,
        browser: Browser,
        login_url: &str,
        account_name: &str,
    ) -> (Browser, Result<HashMap<String, String>>) {
        // Create new page
        let page = match browser.new_page("about:blank").await {
            Ok(p) => p,
            Err(e) => {
                let err_msg = format!("Failed to create new page: {}", e);
                log::error!("[{}] {}", account_name, err_msg);
                return (browser, Err(anyhow::anyhow!(err_msg)));
            }
        };

        info!("[{}] New page created", account_name);

        // Set user agent
        if let Err(e) = page.set_user_agent(USER_AGENT).await {
            let err_msg = format!("Failed to set user agent: {}", e);
            log::error!("[{}] {}", account_name, err_msg);
            return (browser, Err(anyhow::anyhow!(err_msg)));
        }

        info!("[{}] Navigating to: {}", account_name, login_url);

        // Navigate to login page
        if let Err(e) = page.goto(login_url).await {
            let err_msg = format!("Failed to navigate to login page: {}", e);
            log::error!("[{}] {}", account_name, err_msg);
            return (browser, Err(anyhow::anyhow!(err_msg)));
        }

        info!("[{}] Page loaded, waiting for WAF cookies...", account_name);

        // Wait for cookies to be set (configurable timeout)
        let timeout_config = TimeoutConfig::global();
        tokio::time::sleep(timeout_config.waf_wait).await;

        // Get all cookies
        let cookies = match page.get_cookies().await {
            Ok(c) => c,
            Err(e) => {
                let err_msg = format!("Failed to get cookies: {}", e);
                log::error!("[{}] {}", account_name, err_msg);
                return (browser, Err(anyhow::anyhow!(err_msg)));
            }
        };

        info!(
            "[{}] Retrieved {} cookies from browser",
            account_name,
            cookies.len()
        );

        // Extract WAF cookies
        let mut waf_cookies = HashMap::new();
        for cookie in cookies {
            let cookie_name = &cookie.name;
            let cookie_value = &cookie.value;

            info!(
                "[{}] Cookie found: {} = {}",
                account_name,
                cookie_name,
                mask_sensitive(cookie_value)
            );

            if REQUIRED_WAF_COOKIES.contains(&cookie_name.as_str()) {
                waf_cookies.insert(cookie_name.clone(), cookie_value.clone());
                info!("[{}] ✓ WAF cookie captured: {}", account_name, cookie_name);
            }
        }

        info!(
            "[{}] Captured {} WAF cookies out of {} required",
            account_name,
            waf_cookies.len(),
            REQUIRED_WAF_COOKIES.len()
        );

        (browser, Ok(waf_cookies))
    }
}
