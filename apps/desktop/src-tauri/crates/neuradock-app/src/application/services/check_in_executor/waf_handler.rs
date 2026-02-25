use log::{error, info, warn};
use std::collections::HashMap;

use neuradock_domain::{account::Account, check_in::Provider};
use neuradock_infrastructure::http::{CheckInResult, HttpClient, SetCookieResult};

use super::execution::create_error_result;
use crate::application::services::waf_cookie_manager::WafCookieManager;

/// Retry check-in after refreshing WAF cookies
#[allow(clippy::too_many_arguments)]
pub async fn retry_check_in_after_waf_refresh(
    waf_manager: &WafCookieManager,
    http_client: &HttpClient,
    account: &Account,
    provider: &Provider,
    account_name: &str,
    sign_in_url: &str,
    cookies: &mut HashMap<String, String>,
    api_user: &str,
) -> (CheckInResult, SetCookieResult) {
    warn!(
        "[{}] WAF challenge detected during check-in, refreshing cookies and retrying...",
        account_name
    );

    // Refresh WAF cookies
    let fresh_cookies = match waf_manager
        .refresh_waf_cookies(account_name, provider, account.credentials().cookies())
        .await
    {
        Ok(fresh) => fresh,
        Err(refresh_err) => {
            error!(
                "[{}] Failed to refresh WAF cookies: {}",
                account_name, refresh_err
            );
            return (
                create_error_result(&format!("WAF refresh failed: {}", refresh_err)),
                SetCookieResult::default(),
            );
        }
    };

    // Update cookies for subsequent operations
    *cookies = fresh_cookies;

    // Retry check-in with fresh cookies
    match http_client
        .execute_check_in(sign_in_url, cookies, provider.api_user_key(), api_user)
        .await
    {
        Ok((result, set_cookies)) => {
            info!(
                "[{}] Check-in retry successful after WAF refresh!",
                account_name
            );
            (result, set_cookies)
        }
        Err(retry_err) => {
            error!("[{}] Check-in retry failed: {}", account_name, retry_err);
            (
                create_error_result(&format!("Check-in failed after WAF retry: {}", retry_err)),
                SetCookieResult::default(),
            )
        }
    }
}
