use anyhow::{Context, Result};
use reqwest::{header, Client};
use std::collections::HashMap;

use super::types::{extract_domain, extract_set_cookies, SetCookieResult, UserInfo};

impl super::HttpClient {
    /// Get user info (quota and used quota) with retry
    pub async fn get_user_info(
        &self,
        url: &str,
        cookies: &HashMap<String, String>,
        api_user_key: &str,
        api_user_value: &str,
    ) -> Result<(UserInfo, SetCookieResult)> {
        let url = url.to_string();
        let cookies = cookies.clone();
        let api_user_key = api_user_key.to_string();
        let api_user_value = api_user_value.to_string();

        self.execute_with_retry("Get user info", move || {
            let url = url.clone();
            let cookies = cookies.clone();
            let api_user_key = api_user_key.clone();
            let api_user_value = api_user_value.clone();
            let client = self.client.clone();

            async move {
                Self::get_user_info_once(&client, &url, &cookies, &api_user_key, &api_user_value)
                    .await
            }
        })
        .await
    }

    /// Get user info (quota and used quota) - single attempt
    async fn get_user_info_once(
        client: &Client,
        url: &str,
        cookies: &HashMap<String, String>,
        api_user_key: &str,
        api_user_value: &str,
    ) -> Result<(UserInfo, SetCookieResult)> {
        // Build headers
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json, text/plain, */*"),
        );
        headers.insert(
            header::ACCEPT_LANGUAGE,
            header::HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8"),
        );
        headers.insert(
            header::REFERER,
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
        let mut request = client.get(url).headers(headers);

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
            .context("Failed to send user info request")?;

        let status = response.status();
        log::info!("User info response status: {}", status);

        // Extract Set-Cookie headers before consuming the response body
        let set_cookie_result = extract_set_cookies(&response);

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read response".to_string());
            log::error!(
                "User info request failed with status {}: {}",
                status,
                &error_text[..error_text.len().min(500)]
            );

            // Try to extract message from JSON response
            let error_message =
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&error_text) {
                    json["message"].as_str().unwrap_or(&error_text).to_string()
                } else {
                    error_text.clone()
                };

            // Check if this is a WAF challenge
            if error_text.contains("acw_sc__v2") || error_text.contains("<script>var arg1=") {
                anyhow::bail!(
                    "WAF_CHALLENGE: {}",
                    &error_text[..error_text.len().min(500)]
                );
            }
            anyhow::bail!("{}", error_message);
        }

        // Get response text first to check for WAF challenge
        let response_text = response
            .text()
            .await
            .context("Failed to read user info response")?;

        log::debug!("User info response length: {} bytes", response_text.len());

        // Check if response is HTML (WAF challenge page)
        if response_text.trim().starts_with('<')
            || response_text.contains("acw_sc__v2")
            || response_text.contains("<script>var arg1=")
        {
            log::warn!(
                "Received WAF challenge page instead of JSON: {}",
                &response_text[..response_text.len().min(200)]
            );
            anyhow::bail!(
                "WAF_CHALLENGE: Received HTML instead of JSON - {}",
                &response_text[..response_text.len().min(500)]
            );
        }

        let data: serde_json::Value = serde_json::from_str(&response_text).context(format!(
            "Failed to parse user info response: {}",
            &response_text[..response_text.len().min(200)]
        ))?;

        // Debug: Log the full response to understand structure
        log::info!(
            "User info API response: {}",
            serde_json::to_string_pretty(&data)
                .unwrap_or_else(|_| "failed to serialize".to_string())
        );

        // Check if response has expected structure
        if data["data"].is_null() {
            anyhow::bail!("API response missing 'data' field: {}", data);
        }

        // Parse quota and used_quota (convert from bytes to dollars, 500000 bytes = $1)
        let quota_bytes = data["data"]["quota"]
            .as_f64()
            .ok_or_else(|| anyhow::anyhow!("Missing or invalid 'quota' field in API response"))?;
        let used_quota_bytes = data["data"]["used_quota"].as_f64().ok_or_else(|| {
            anyhow::anyhow!("Missing or invalid 'used_quota' field in API response")
        })?;

        let current_balance = (quota_bytes / 500000.0 * 100.0).round() / 100.0;
        let total_consumed = (used_quota_bytes / 500000.0 * 100.0).round() / 100.0;
        let total_quota = current_balance + total_consumed;

        // NOTE: Upstream's HTTP payload still calls `quota`, `used_quota`, and `total_income`.
        // We normalize semantics right here so the rest of the app only deals with
        // `current_balance`, `total_consumed`, and `total_quota` to avoid confusion.
        Ok((UserInfo {
            current_balance,
            total_consumed,
            total_quota,
        }, set_cookie_result))
    }
}
