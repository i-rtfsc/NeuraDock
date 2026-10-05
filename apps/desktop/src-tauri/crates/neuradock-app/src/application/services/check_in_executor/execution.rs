use log::{error, info};
use std::collections::HashMap;

use neuradock_infrastructure::http::{check_in_codes, CheckInResult, HttpClient, SetCookieResult};

/// Execute check-in via page visit
pub async fn execute_page_visit_check_in(
    http_client: &HttpClient,
    account_name: &str,
    sign_in_url: &str,
    cookies: &HashMap<String, String>,
) -> (CheckInResult, SetCookieResult) {
    info!("[{}] Visiting check-in page: {}", account_name, sign_in_url);

    match http_client.visit_login_page(sign_in_url, cookies).await {
        Ok(set_cookies) => {
            info!("[{}] Check-in page visited successfully!", account_name);
            (
                CheckInResult::failure(
                    check_in_codes::INVALID_RESPONSE,
                    "Page visited, but the provider has not confirmed check-in",
                ),
                set_cookies,
            )
        }
        Err(e) => {
            error!("[{}] Failed to visit check-in page: {}", account_name, e);
            (
                create_error_result(
                    check_in_codes::NETWORK,
                    &format!("Failed to visit page: {}", e),
                ),
                SetCookieResult::default(),
            )
        }
    }
}

/// Execute check-in via API call with WAF retry logic
pub async fn execute_api_check_in(
    http_client: &HttpClient,
    sign_in_url: &str,
    cookies: &HashMap<String, String>,
    api_user_key: &str,
    api_user: &str,
    account_name: &str,
) -> anyhow::Result<(CheckInResult, SetCookieResult)> {
    let (result, set_cookies) = http_client
        .execute_check_in(sign_in_url, cookies, api_user_key, api_user)
        .await?;

    if result.success {
        info!("[{}] Check-in successful!", account_name);
    } else {
        log::warn!("[{}] Check-in failed: {}", account_name, result.message);
    }

    Ok((result, set_cookies))
}

/// Create a failed result carrying a stable error code.
pub fn create_error_result(code: &str, message: &str) -> CheckInResult {
    CheckInResult::failure(code, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_error_result() {
        let result = create_error_result(check_in_codes::NETWORK, "Test error message");

        assert!(!result.success);
        assert_eq!(result.message, "Test error message");
        assert_eq!(result.error_code(), Some(check_in_codes::NETWORK));
    }

    #[test]
    fn test_create_error_result_empty_message() {
        let result = create_error_result(check_in_codes::TIMEOUT, "");

        assert!(!result.success);
        assert_eq!(result.message, "");
    }
}
