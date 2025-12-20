use anyhow::Result;

use super::types::{FetchTokensRequest, TokenResponse};

impl super::TokenClient {
    pub async fn fetch_tokens(&self, request: FetchTokensRequest<'_>) -> Result<TokenResponse> {
        let url = format!(
            "{}?p={}&size={}",
            Self::build_url(request.base_url, request.token_api_path),
            request.page,
            request.size
        );
        let normalized_base = request.base_url.trim_end_matches('/');

        log::info!("Fetching tokens from: {}", url);
        log::debug!(
            "Cookie length: {}, API user: {:?}",
            request.cookie_string.len(),
            request.api_user
        );

        let mut http_request = self
            .client
            .get(&url)
            .header("Cookie", request.cookie_string)
            .header("Accept", "application/json")
            .header("Accept-Encoding", "gzip, deflate, br")
            .header("Cache-Control", "no-store")
            .header("Referer", format!("{}/console/token", normalized_base));

        if let Some(user) = request.api_user {
            let header_name = request.api_user_header.unwrap_or("New-API-User");
            log::debug!("Adding {} header: {}", header_name, user);
            http_request = http_request.header(header_name, user);
        }

        let response = http_request.send().await?;

        if !response.status().is_success() {
            log::error!("HTTP request failed: {}", response.status());
            anyhow::bail!("Failed to fetch tokens: HTTP {}", response.status());
        }

        log::debug!(
            "Response status: {}, headers: {:?}",
            response.status(),
            response.headers()
        );

        // Read response text first for debugging
        let response_text = response.text().await?;
        log::debug!("Response body: {}", response_text);

        // Check if response is WAF challenge page
        if response_text.contains("<html>") && response_text.contains("acw_sc__v2") {
            log::warn!("Detected WAF challenge page, cookies may be expired or invalid");
            anyhow::bail!("WAF_CHALLENGE: Session cookies expired or invalid, please re-login to refresh WAF cookies");
        }

        // Parse JSON
        let token_response: TokenResponse = serde_json::from_str(&response_text).map_err(|e| {
            log::error!("Failed to parse JSON: {}", e);
            log::error!("Response text was: {}", response_text);
            anyhow::anyhow!("Failed to parse response: {}", e)
        })?;

        if !token_response.success {
            log::error!("API returned error: {}", token_response.message);
            anyhow::bail!("API returned error: {}", token_response.message);
        }

        log::info!(
            "Successfully fetched {} tokens (page {}, total: {})",
            token_response.data.items().len(),
            token_response.data.page(),
            token_response.data.total()
        );

        Ok(token_response)
    }
}
