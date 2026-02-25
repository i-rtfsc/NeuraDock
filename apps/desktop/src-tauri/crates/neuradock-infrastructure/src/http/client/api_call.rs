use anyhow::{Context, Result};
use reqwest::header;
use std::collections::HashMap;

use super::types::{extract_set_cookies, SetCookieResult};

impl super::HttpClient {
    /// Call API endpoint with GET request (for triggering balance updates)
    pub async fn call_api_endpoint(
        &self,
        url: &str,
        cookies: &HashMap<String, String>,
        api_user_key: &str,
        api_user_value: &str,
    ) -> Result<SetCookieResult> {
        log::info!("Calling API endpoint: {}", url);

        // Build headers
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json, text/plain, */*"),
        );
        headers.insert(
            header::HeaderName::from_static("x-requested-with"),
            header::HeaderValue::from_static("XMLHttpRequest"),
        );

        // Add API user header
        if !api_user_value.is_empty() {
            headers.insert(
                header::HeaderName::from_bytes(api_user_key.as_bytes())?,
                header::HeaderValue::from_str(api_user_value)?,
            );
        }

        // Build request with cookies
        let mut request = self.client.get(url).headers(headers);

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
            .context("Failed to call API endpoint")?;

        let status = response.status();
        log::info!("API endpoint response status: {}", status);

        // Extract Set-Cookie headers before consuming the response
        let set_cookie_result = extract_set_cookies(&response);

        // Get response text to check for WAF challenge
        let response_text = response.text().await.unwrap_or_else(|_| String::new());

        // Check for WAF challenge
        if response_text.contains("acw_sc__v2") || response_text.contains("<script>var arg1=") {
            log::warn!("WAF challenge detected in API endpoint response");
            anyhow::bail!(
                "WAF_CHALLENGE: {}",
                &response_text[..response_text.len().min(500)]
            );
        }

        if !status.is_success() {
            log::warn!(
                "API endpoint returned non-success status {}: {}",
                status,
                &response_text[..response_text.len().min(200)]
            );
            // Don't fail, just log warning
        }

        Ok(set_cookie_result)
    }
}
