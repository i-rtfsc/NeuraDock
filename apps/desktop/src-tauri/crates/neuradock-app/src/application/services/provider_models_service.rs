use log::{error, info};
use neuradock_domain::check_in::Provider;
use neuradock_infrastructure::http::token::TokenClient;
use neuradock_infrastructure::persistence::repositories::{
    SqliteProviderModelsRepository, SqliteWafCookiesRepository,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Service for fetching and saving provider models
pub struct ProviderModelsService {
    provider_models_repo: Arc<SqliteProviderModelsRepository>,
    waf_cookies_repo: Arc<SqliteWafCookiesRepository>,
}

impl ProviderModelsService {
    pub fn new(
        provider_models_repo: Arc<SqliteProviderModelsRepository>,
        waf_cookies_repo: Arc<SqliteWafCookiesRepository>,
    ) -> Self {
        Self {
            provider_models_repo,
            waf_cookies_repo,
        }
    }

    /// Fetch and save provider models after successful check-in
    pub async fn fetch_and_save_provider_models(
        &self,
        provider: &Provider,
        cookies: &HashMap<String, String>,
        api_user: &str,
    ) {
        // Check if provider has models API
        let models_path = match provider.models_path() {
            Some(path) => path,
            None => {
                info!(
                    "Provider {} does not have models API, skipping",
                    provider.name()
                );
                return;
            }
        };

        let provider_id = provider.id().as_str();
        let base_url = provider.domain().trim_end_matches('/').to_string();
        let api_user_header = provider.api_user_key();
        let api_user_header_opt = if api_user_header.is_empty() {
            None
        } else {
            Some(api_user_header)
        };

        // Build cookies with WAF cookies from cache
        let mut all_cookies = cookies.clone();

        // Try to get cached WAF cookies
        match self.waf_cookies_repo.get_valid(provider_id).await {
            Ok(Some(cached_waf)) => {
                info!("Using cached WAF cookies for provider models fetch");
                all_cookies.extend(cached_waf.cookies);
            }
            Ok(None) => {
                info!("No cached WAF cookies available for provider models fetch");
            }
            Err(e) => {
                error!("Failed to get cached WAF cookies: {}", e);
            }
        }

        // Build cookie string
        let cookie_string: String = all_cookies
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ");

        // Create token client and fetch models
        let client = match TokenClient::new() {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to create token client: {}", e);
                return;
            }
        };

        // Use account's api_user value, not provider's api_user_key (which is the header name)
        let api_user_opt = if api_user.is_empty() {
            None
        } else {
            Some(api_user)
        };

        match client
            .fetch_provider_models(
                &base_url,
                models_path,
                &cookie_string,
                api_user_header_opt,
                api_user_opt,
            )
            .await
        {
            Ok(models) => {
                info!(
                    "Fetched {} models for provider {}",
                    models.len(),
                    provider.name()
                );

                // Save to database
                if let Err(e) = self.provider_models_repo.save(provider_id, &models).await {
                    error!("Failed to save provider models: {}", e);
                } else {
                    info!("Provider models saved to database");
                }
            }
            Err(e) => {
                // Don't fail the check-in if models fetch fails
                error!("Failed to fetch provider models (non-critical): {}", e);
            }
        }
    }

    /// Check if provider models should be auto-fetched
    pub async fn should_auto_fetch(&self, provider_id: &str) -> bool {
        match self.provider_models_repo.find_by_provider(provider_id).await {
            Ok(existing_models) => {
                existing_models.is_none()
                    || existing_models
                        .as_ref()
                        .map(|m| m.models.is_empty())
                        .unwrap_or(true)
            }
            Err(_) => false,
        }
    }
}
