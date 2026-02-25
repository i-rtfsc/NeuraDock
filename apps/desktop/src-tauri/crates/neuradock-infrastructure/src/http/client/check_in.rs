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
                anyhow::bail!(
                    "WAF_CHALLENGE: {}",
                    &error_text[..error_text.len().min(500)]
                );
            }
            log::error!(
                "Check-in request failed with status {}: {}",
                status,
                error_text
            );
            anyhow::bail!(
                "Check-in request failed with status {}: {}",
                status,
                error_text
            );
        }

        // Parse response
        let text = response.text().await?;

        // Check if response is HTML (WAF challenge page)
        if text.trim().starts_with('<')
            || text.contains("acw_sc__v2")
            || text.contains("<script>var arg1=")
        {
            log::warn!(
                "Received WAF challenge page instead of JSON in check-in: {}",
                &text[..text.len().min(200)]
            );
            anyhow::bail!(
                "WAF_CHALLENGE: Received HTML instead of JSON - {}",
                &text[..text.len().min(500)]
            );
        }

        // Log full response for debugging
        log::info!("Check-in response body: {}", text);

        // Try to parse as JSON
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
            log::info!(
                "Parsed JSON response: {}",
                serde_json::to_string_pretty(&data).unwrap_or_default()
            );

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
            let success = ret_value == Some(1)
                || code_value == Some(0)
                || code_value == Some(200)
                || success_value == Some(true);

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

            Ok((CheckInResult { success, message }, set_cookie_result))
        } else {
            log::warn!("Failed to parse as JSON, raw response: {}", text);

            // If not JSON, check if response contains "success"
            let success = text.to_lowercase().contains("success");
            let message = if success {
                "Check-in successful".to_string()
            } else {
                format!(
                    "Check-in failed: Invalid response format. Response: {}",
                    if text.len() > 100 {
                        &text[..100]
                    } else {
                        &text
                    }
                )
            };

            Ok((CheckInResult { success, message }, set_cookie_result))
        }
    }
}
