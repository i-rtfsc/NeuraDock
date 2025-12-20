mod api_call;
mod check_in;
mod types;
mod user_info;
mod visit;

pub use types::{CheckInResult, RetryConfig, UserInfo};

use anyhow::{Context, Result};
use log::{debug, warn};
use reqwest::{Client, StatusCode};
use std::time::Duration;
use tokio::time::sleep;

use types::USER_AGENT;

pub struct HttpClient {
    pub(super) client: Client,
    pub(super) retry_config: RetryConfig,
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
    pub(super) async fn execute_with_retry<F, Fut, T>(
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
}

impl Default for HttpClient {
    fn default() -> Self {
        match Self::new() {
            Ok(client) => client,
            Err(e) => {
                warn!("⚠️  Failed to create default HTTP client: {}", e);
                Self {
                    client: Client::new(),
                    retry_config: RetryConfig::default(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_client_creation() {
        let client = HttpClient::new();
        assert!(client.is_ok());
    }
}
