use anyhow::{Context, Result};
use reqwest::header;
use std::collections::HashMap;

use super::types::{extract_domain, extract_set_cookies, CheckInResult, SetCookieResult};

impl super::HttpClient {
    /// Execute check-in with retry logic
    pub async fn execute_check_in(
        &self,
        url: &str,
        cookies: &HashMap<String, String>,
        api_user_key: &str,
        api_user_value: &str,
    ) -> Result<(CheckInResult, SetCookieResult)> {
        const MAX_RETRIES: u32 = 3;
        const INITIAL_DELAY_MS: u64 = 1000;

        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                let delay = INITIAL_DELAY_MS * 2u64.pow(attempt - 1);
                log::info!(
                    "Retrying check-in (attempt {}/{}), waiting {}ms...",
                    attempt + 1,
                    MAX_RETRIES,
                    delay
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }

            match self
                .execute_check_in_once(url, cookies, api_user_key, api_user_value)
                .await
            {
                Ok(result) => return Ok(result),
                Err(e) => {
                    log::warn!("Check-in attempt {} failed: {}", attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| anyhow::anyhow!("Check-in failed after {} attempts", MAX_RETRIES)))
    }

    /// Execute check-in once (internal method)
    async fn execute_check_in_once(
        &self,
        url: &str,
        cookies: &HashMap<String, String>,
        api_user_key: &str,
        api_user_value: &str,
    ) -> Result<(CheckInResult, SetCookieResult)> {
        // Build headers
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json, text/plain, */*"),
        );
        headers.insert(
            header::HeaderName::from_static("x-requested-with"),
            header::HeaderValue::from_static("XMLHttpRequest"),
        );
        headers.insert(
            header::REFERER,
            header::HeaderValue::from_str(&extract_domain(url)?)?,
        );
        headers.insert(
            header::ORIGIN,
            header::HeaderValue::from_str(&extract_domain(url)?)?,
        );

        // Add API user header
        if !api_user_value.is_empty() {
            headers.insert(
                header::HeaderName::from_bytes(api_user_key.as_bytes())?,
                header::HeaderValue::from_str(api_user_value)?,
            );
        }

        // Build request with cookies
        let mut request = self.client.post(url).headers(headers);

        // Add cookies as header string
        let cookie_string = cookies
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ");

        if !cookie_string.is_empty() {
            request = request.header(header::COOKIE, cookie_string);
        }

        // Send request
        let response = request
            .send()
            .await
            .context("Failed to send check-in request")?;

        let status = response.status();

        log::info!("Check-in response status: {}", status);

        // Extract Set-Cookie headers before consuming the response body
        let set_cookie_result = extract_set_cookies(&response);

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read response".to_string());
            // Check if this is a WAF challenge
            if error_text.contains("acw_sc__v2") || error_text.contains("<script>var arg1=") {
                // The body is logged in truncated form for diagnosis only; the
                // error that reaches callers and the UI carries no body.
                log::warn!(
                    "Check-in hit a WAF challenge page ({} bytes)",
                    error_text.len()
                );
                anyhow::bail!("WAF_CHALLENGE: the provider returned a challenge page");
            }
            log::error!(
                "Check-in request failed with status {}: {}",
                status,
                crate::logging::log_utils::truncate_for_log(&error_text, 200)
            );
            // A truncated body still echoes account-identifying fields and
            // provider HTML, so callers get a status class only. The class
            // decides whether retrying today can help.
            let code = if status.is_client_error() {
                crate::http::client::types::check_in_codes::HTTP_CLIENT
            } else {
                crate::http::client::types::check_in_codes::HTTP_SERVER
            };
            anyhow::bail!("{code}: check-in request was rejected with {status}");
        }

        // Parse response
        let text = response.text().await?;

        // Check if response is HTML (WAF challenge page)
        if text.trim().starts_with('<')
            || text.contains("acw_sc__v2")
            || text.contains("<script>var arg1=")
        {
            log::warn!(
                "Received WAF challenge page instead of JSON in check-in ({} bytes)",
                text.len()
            );
            anyhow::bail!("WAF_CHALLENGE: the provider returned an HTML challenge instead of JSON");
        }

        // Body is not logged: check-in responses echo account-identifying
        // fields, and this ran at info! level (on by default).
        log::debug!("Check-in response body: {} bytes", text.len());

        // Try to parse as JSON
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
            // Check for success indicators with more detailed logging
            let ret_value = data["ret"].as_i64();
            let code_value = data["code"].as_i64();
            let success_value = data["success"].as_bool();

            log::info!(
                "Success indicators - ret: {:?}, code: {:?}, success: {:?}",
                ret_value,
                code_value,
                success_value
            );

            // Updated success check to handle more cases
            let success = confirmed_success(&data);

            let message = if success {
                data["msg"]
                    .as_str()
                    .or(data["message"].as_str())
                    .or(data["data"].as_str())
                    .unwrap_or("Check-in successful")
                    .to_string()
            } else {
                let error_msg = data["msg"]
                    .as_str()
                    .or(data["message"].as_str())
                    .or(data["error"].as_str())
                    .unwrap_or("Check-in failed");
                log::warn!("Check-in failed: {}", error_msg);
                error_msg.to_string()
            };

            let reward = if success { parse_reward(&data) } else { None };

            if !success {
                // "You already checked in today" is not a failure: it is the
                // provider confirming the day is satisfied. Recording it beats
                // guessing from a quota delta, and it keeps a duplicate request
                // from looking like an error.
                if already_checked_in(&message) {
                    log::info!("Provider reports the check-in was already done");
                    return Ok((CheckInResult::already_checked_in(), set_cookie_result));
                }
                log::warn!("Check-in rejected by the provider");
                return Ok((
                    CheckInResult::failure(
                        crate::http::client::types::check_in_codes::INVALID_RESPONSE,
                        message,
                    ),
                    set_cookie_result,
                ));
            }

            Ok((CheckInResult::confirmed(message, reward), set_cookie_result))
        } else {
            // No body in the log: check-in responses echo account-identifying
            // fields.
            log::warn!("Check-in response was not JSON ({} bytes)", text.len());

            // A page containing the word "success" is not a confirmed reward.
            Ok((
                CheckInResult::failure(
                    crate::http::client::types::check_in_codes::INVALID_RESPONSE,
                    "Check-in could not be confirmed: invalid response format",
                ),
                set_cookie_result,
            ))
        }
    }
}

/// Provider sentences that mean "this quota day is already done".
///
/// Kept deliberately narrow and explicit. A false positive here invents a
/// check-in, which is worse than a false negative: the balance-delta inference
/// still has a chance to catch the day.
fn already_checked_in(message: &str) -> bool {
    let lowered = message.to_lowercase();
    [
        "已经签到",
        "已签到",
        "重复签到",
        "今日已签",
        "already checked in",
        "already signed in",
    ]
    .iter()
    .any(|needle| lowered.contains(&needle.to_lowercase()))
}

fn confirmed_success(data: &serde_json::Value) -> bool {
    if let Some(success) = data["success"].as_bool() {
        return success;
    }
    if let Some(ret) = data["ret"].as_i64() {
        return ret == 1;
    }
    matches!(data["code"].as_i64(), Some(0 | 200))
}

fn parse_reward(data: &serde_json::Value) -> Option<f64> {
    let dollars = data
        .pointer("/data/reward_dollars")
        .or_else(|| data.get("reward_dollars"))
        .and_then(serde_json::Value::as_f64)
        .or_else(|| {
            data.pointer("/data/quota_awarded")
                .or_else(|| data.get("quota_awarded"))
                .and_then(serde_json::Value::as_f64)
                .map(|units| units / 500_000.0)
        })?;
    (dollars.is_finite() && (0.0..=1e12).contains(&dollars))
        .then_some((dollars * 100.0).round() / 100.0)
}

#[cfg(test)]
mod reward_tests {
    use super::{confirmed_success, parse_reward};
    use serde_json::json;

    #[test]
    fn an_explicit_failure_is_not_overridden_by_a_success_shaped_status_code() {
        assert!(!confirmed_success(&json!({"success":false,"code":200})));
        assert!(!confirmed_success(&json!({"ret":0,"code":0})));
        assert!(!confirmed_success(&json!({"message":"success"})));
        assert!(confirmed_success(&json!({"success":true})));
        assert!(confirmed_success(&json!({"ret":1})));
    }

    #[test]
    fn parses_explicit_rewards_without_mistaking_account_balance_for_reward() {
        assert_eq!(
            parse_reward(&json!({"data":{"quota_awarded":12_500_000}})),
            Some(25.0)
        );
        assert_eq!(
            parse_reward(&json!({"data":{"reward_dollars":2.5}})),
            Some(2.5)
        );
        assert_eq!(
            parse_reward(&json!({"data":{"quota":12_500_000,"used_quota":42}})),
            None
        );
        assert_eq!(parse_reward(&json!({"data":{"quota_awarded":-1}})), None);
        assert_eq!(parse_reward(&json!({"message":"success"})), None);
    }
}
