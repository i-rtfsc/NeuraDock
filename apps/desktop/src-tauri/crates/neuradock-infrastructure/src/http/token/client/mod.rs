mod models;
mod tokens;
mod types;

use anyhow::Result;
use reqwest::Client;

// Re-export types
pub use types::{FetchTokensRequest, TokenData, TokenResponse};

pub struct TokenClient {
    pub(super) client: Client,
}

impl TokenClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .gzip(true) // Enable automatic gzip decompression
            .build()?;

        Ok(Self { client })
    }

    pub(super) fn build_url(base: &str, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            format!(
                "{}/{}",
                base.trim_end_matches('/'),
                path.trim_start_matches('/')
            )
        }
    }
}

impl Default for TokenClient {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self { client: Client::new() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_client_creation() {
        let client = TokenClient::new();
        assert!(client.is_ok());
    }
}
