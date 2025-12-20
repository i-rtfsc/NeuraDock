use anyhow::Result;

use super::types::ProviderModelsResponse;

impl super::TokenClient {
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
}
