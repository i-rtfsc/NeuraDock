use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub success: bool,
    pub message: String,
    #[serde(flatten)]
    pub data: TokenResponseData,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TokenResponseData {
    /// AgentRouter format: {"data": {"page": 1, "items": [...]}}
    Paginated { data: TokenDataWrapper },
    /// AnyRouter format: {"data": [...]}
    Direct { data: Vec<TokenData> },
}

#[derive(Debug, Deserialize)]
pub struct TokenDataWrapper {
    pub page: u32,
    pub page_size: u32,
    pub total: u32,
    pub items: Vec<TokenData>,
}

impl TokenResponseData {
    pub fn items(&self) -> &[TokenData] {
        match self {
            TokenResponseData::Paginated { data } => &data.items,
            TokenResponseData::Direct { data } => data,
        }
    }

    pub fn page(&self) -> u32 {
        match self {
            TokenResponseData::Paginated { data } => data.page,
            TokenResponseData::Direct { .. } => 1,
        }
    }

    pub fn total(&self) -> u32 {
        match self {
            TokenResponseData::Paginated { data } => data.total,
            TokenResponseData::Direct { data } => data.len() as u32,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenData {
    pub id: i64,
    pub user_id: i64,
    pub key: String,
    pub status: i32,
    pub name: String,
    pub created_time: i64,
    pub accessed_time: i64,
    pub expired_time: i64,
    pub remain_quota: i64,
    pub unlimited_quota: bool,
    pub used_quota: i64,
    pub model_limits_enabled: bool,
    pub model_limits: serde_json::Value,
}

pub struct TokenClient {
    client: Client,
}

/// Response format for provider models API
#[derive(Debug, Deserialize)]
pub struct ProviderModelsResponse {
    pub success: bool,
    pub message: String,
    pub data: Vec<String>, // Changed: data is a simple string array, not objects
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderModelData {
    pub id: String,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub created: Option<i64>,
    #[serde(default)]
    pub owned_by: Option<String>,
}

impl TokenClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .gzip(true) // Enable automatic gzip decompression
            .build()?;

        Ok(Self { client })
    }

    /// Fetch provider supported models from /api/user/models
    pub async fn fetch_provider_models(
        &self,
        base_url: &str,
        models_path: &str,
        cookie_string: &str,
        api_user_header: Option<&str>,
        api_user: Option<&str>,
    ) -> Result<Vec<String>> {
        let url = Self::build_url(base_url, models_path);
        let normalized_base = base_url.trim_end_matches('/');

        log::info!("Fetching provider models from: {}", url);

        let mut request = self
            .client
            .get(&url)
            .header("Cookie", cookie_string)
            .header("Accept", "application/json")
            .header("Accept-Encoding", "gzip, deflate, br")
            .header("Cache-Control", "no-store")
            .header("Referer", format!("{}/console", normalized_base));

        if let Some(user) = api_user {
            let header_name = api_user_header.unwrap_or("New-API-User");
            request = request.header(header_name, user);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            log::error!("HTTP request failed: {}", response.status());
            anyhow::bail!("Failed to fetch models: HTTP {}", response.status());
        }

        let response_text = response.text().await?;
        log::debug!("Models response: {}", response_text);

        // Check if response is WAF challenge page
        if response_text.contains("<html>") && response_text.contains("acw_sc__v2") {
            log::warn!("Detected WAF challenge page");
            anyhow::bail!("WAF_CHALLENGE: Session cookies expired or invalid");
        }

        // Parse JSON
        let models_response: ProviderModelsResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                log::error!("Failed to parse models JSON: {}", e);
                anyhow::anyhow!("Failed to parse models response: {}", e)
            })?;

        if !models_response.success {
            log::error!("API returned error: {}", models_response.message);
            anyhow::bail!("API returned error: {}", models_response.message);
        }

        // Data is already a Vec<String>, no need to extract
        let model_ids = models_response.data;

        log::info!("Successfully fetched {} models", model_ids.len());

        Ok(model_ids)
    }

    pub async fn fetch_tokens(
        &self,
        base_url: &str,
        token_api_path: &str,
        cookie_string: &str,
        api_user_header: Option<&str>,
        api_user: Option<&str>,
        page: u32,
        size: u32,
    ) -> Result<TokenResponse> {
        let url = format!(
            "{}?p={}&size={}",
            Self::build_url(base_url, token_api_path),
            page,
            size
        );
        let normalized_base = base_url.trim_end_matches('/');

        log::info!("Fetching tokens from: {}", url);
        log::debug!(
            "Cookie length: {}, API user: {:?}",
            cookie_string.len(),
            api_user
        );

        let mut request = self
            .client
            .get(&url)
            .header("Cookie", cookie_string)
            .header("Accept", "application/json")
            .header("Accept-Encoding", "gzip, deflate, br")
            .header("Cache-Control", "no-store")
            .header("Referer", format!("{}/console/token", normalized_base));

        if let Some(user) = api_user {
            let header_name = api_user_header.unwrap_or("New-API-User");
            log::debug!("Adding {} header: {}", header_name, user);
            request = request.header(header_name, user);
        }

        let response = request.send().await?;

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

    fn build_url(base: &str, path: &str) -> String {
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
        Self::new().expect("Failed to create TokenClient")
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
