use anyhow::{Context, Result};
use log::{debug, warn};
use reqwest::{header, Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36";

/// HTTP retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (default: 3)
    pub max_retries: u32,
    /// Initial backoff duration in milliseconds (default: 1000ms)
    pub initial_backoff_ms: u64,
    /// Maximum backoff duration in milliseconds (default: 10000ms)
    pub max_backoff_ms: u64,
    /// Backoff multiplier (default: 2.0 for exponential backoff)
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 1000,
            max_backoff_ms: 10000,
            backoff_multiplier: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub quota: f64,
    pub used_quota: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckInResult {
    pub success: bool,
    pub message: String,
}

pub struct HttpClient {
    client: Client,
    retry_config: RetryConfig,
}

impl HttpClient {
    pub fn new() -> Result<Self> {
        Self::with_retry_config(RetryConfig::default())
    }

    pub fn with_retry_config(retry_config: RetryConfig) -> Result<Self> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .cookie_store(true)
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            retry_config,
        })
    }

    /// Execute a request with retry logic
    ///
    /// Retries on:
    /// - Network errors (connection failures, timeouts)
    /// - 5xx server errors
    /// - 429 Too Many Requests
    ///
    /// Does NOT retry on:
    /// - 4xx client errors (except 429)
    /// - Successful responses (2xx, 3xx)
    async fn execute_with_retry<F, Fut, T>(
        &self,
        operation_name: &str,
        mut request_fn: F,
    ) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let mut backoff_ms = self.retry_config.initial_backoff_ms;

        loop {
            attempt += 1;

            match request_fn().await {
                Ok(response) => {
                    if attempt > 1 {
                        debug!("✅ {} succeeded after {} attempts", operation_name, attempt);
                    }
                    return Ok(response);
                }
                Err(e) => {
                    let should_retry =
                        attempt <= self.retry_config.max_retries && self.is_retryable_error(&e);

                    if should_retry {
                        warn!(
                            "⚠️  {} failed (attempt {}/{}): {}. Retrying in {}ms...",
                            operation_name, attempt, self.retry_config.max_retries, e, backoff_ms
                        );

                        sleep(Duration::from_millis(backoff_ms)).await;

                        // Exponential backoff with cap
                        backoff_ms = ((backoff_ms as f64 * self.retry_config.backoff_multiplier)
                            as u64)
                            .min(self.retry_config.max_backoff_ms);
                    } else {
                        if attempt > self.retry_config.max_retries {
                            warn!(
                                "❌ {} failed after {} attempts",
                                operation_name, self.retry_config.max_retries
                            );
                        }
                        return Err(e);
                    }
                }
            }
        }
    }

    /// Check if an error is retryable
    fn is_retryable_error(&self, error: &anyhow::Error) -> bool {
        // Check for reqwest errors
        if let Some(reqwest_err) = error.downcast_ref::<reqwest::Error>() {
            // Retry on network/connection errors
            if reqwest_err.is_connect() || reqwest_err.is_timeout() || reqwest_err.is_request() {
                return true;
            }

            // Retry on 5xx server errors and 429 Too Many Requests
            if let Some(status) = reqwest_err.status() {
                return status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS;
            }
        }

        false
    }

    /// Get user info (quota and used quota) with retry
    pub async fn get_user_info(
        &self,
        url: &str,
        cookies: &HashMap<String, String>,
        api_user_key: &str,
        api_user_value: &str,
    ) -> Result<UserInfo> {
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
    ) -> Result<UserInfo> {
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

        let quota = (quota_bytes / 500000.0 * 100.0).round() / 100.0;
        let used_quota = (used_quota_bytes / 500000.0 * 100.0).round() / 100.0;

        Ok(UserInfo { quota, used_quota })
    }

    /// Execute check-in with retry logic
    pub async fn execute_check_in(
        &self,
        url: &str,
        cookies: &HashMap<String, String>,
        api_user_key: &str,
        api_user_value: &str,
    ) -> Result<CheckInResult> {
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
    ) -> Result<CheckInResult> {
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

            Ok(CheckInResult { success, message })
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

            Ok(CheckInResult { success, message })
        }
    }

    /// Visit login page (for providers that trigger check-in on login page visit)
    pub async fn visit_login_page(
        &self,
        url: &str,
        cookies: &HashMap<String, String>,
    ) -> Result<()> {
        log::info!("Visiting login page: {}", url);

        // Build headers
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static(
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            ),
        );
        headers.insert(
            header::ACCEPT_LANGUAGE,
            header::HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8"),
        );

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

        // Send request (will auto-follow redirects)
        let response = request.send().await.context("Failed to visit login page")?;

        let status = response.status();
        let final_url = response.url().to_string();

        log::info!(
            "Login page visit status: {}, final URL after redirects: {}",
            status,
            final_url
        );

        if !status.is_success() {
            anyhow::bail!("Failed to visit login page, status: {}", status);
        }

        Ok(())
    }

    /// Call API endpoint with GET request (for triggering balance updates)
    pub async fn call_api_endpoint(
        &self,
        url: &str,
        cookies: &HashMap<String, String>,
        api_user_key: &str,
        api_user_value: &str,
    ) -> Result<()> {
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

        Ok(())
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default HTTP client")
    }
}

/// Extract domain from URL (including port if present)
fn extract_domain(url: &str) -> Result<String> {
    let parsed = url::Url::parse(url)?;
    let host = parsed.host_str().unwrap_or("");

    if let Some(port) = parsed.port() {
        Ok(format!("{}://{}:{}", parsed.scheme(), host, port))
    } else {
        Ok(format!("{}://{}", parsed.scheme(), host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            extract_domain("https://example.com/api/user").unwrap(),
            "https://example.com"
        );
        assert_eq!(
            extract_domain("http://test.org:8080/path").unwrap(),
            "http://test.org:8080"
        );
    }

    #[tokio::test]
    async fn test_http_client_creation() {
        let client = HttpClient::new();
        assert!(client.is_ok());
    }
}
