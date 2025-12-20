use crate::presentation::error::CommandError;
use crate::presentation::state::Repositories;
use neuradock_domain::shared::AccountId;
use tauri::State;
/// Fetch provider supported models
/// If forceRefresh is true, will fetch from API regardless of cache
/// Otherwise returns cached models if available and not stale (24 hours)
#[tauri::command]
#[specta::specta]
pub async fn fetch_provider_models(
    provider_id: String,
    account_id: String,
    force_refresh: bool,
    repositories: State<'_, Repositories>,
) -> Result<Vec<String>, CommandError> {
    use neuradock_infrastructure::http::token::TokenClient;

    log::info!(
        "fetch_provider_models: provider_id={}, account_id={}, force_refresh={}",
        provider_id,
        account_id,
        force_refresh
    );

    // Check cache first (unless force refresh)
    if !force_refresh {
        // Check if we have cached models that are not stale (24 hours)
        let is_stale = repositories
            .provider_models
            .is_stale(&provider_id, 24)
            .await
            .map_err(CommandError::from)?;

        if !is_stale {
            if let Some(cached) = repositories
                .provider_models
                .find_by_provider(&provider_id)
                .await
                .map_err(CommandError::from)?
            {
                log::info!(
                    "Returning {} cached models for provider {}",
                    cached.models.len(),
                    provider_id
                );
                return Ok(cached.models);
            }
        }
    }

    // Get account to retrieve cookies
    let account_id_obj = AccountId::from_string(&account_id);
    let account = repositories
        .account
        .find_by_id(&account_id_obj)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found("Account not found"))?;

    // Get provider configuration
    let provider_id_obj = neuradock_domain::shared::ProviderId::from_string(&provider_id);
    let provider = repositories
        .provider
        .find_by_id(&provider_id_obj)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found(format!("Provider not found: {}", provider_id)))?;

    // Check if provider supports models API
    let models_path = provider
        .models_path()
        .ok_or_else(|| CommandError::validation("Provider does not support models API"))?
        .to_string();
    let base_url = provider.domain().trim_end_matches('/').to_string();
    let api_user_header = provider.api_user_key();
    let api_user_header_opt = if api_user_header.is_empty() {
        None
    } else {
        Some(api_user_header)
    };

    // Prepare cookies - start with account cookies
    let mut cookies = account.credentials().cookies().clone();

    // Handle WAF bypass with caching (if provider requires it)
    if provider.needs_waf_bypass() {
        // Check for cached WAF cookies first
        match repositories.waf_cookies.get_valid(&provider_id).await {
            Ok(Some(cached_waf)) => {
                log::info!(
                    "Using cached WAF cookies for provider {} (expires at {})",
                    provider_id,
                    cached_waf.expires_at
                );
                // Merge cached WAF cookies
                for (k, v) in cached_waf.cookies {
                    cookies.insert(k, v);
                }
            }
            Ok(None) => {
                log::warn!(
                    "No valid cached WAF cookies for provider {}, may encounter WAF challenge",
                    provider_id
                );
                // Note: We don't run WAF bypass here to avoid blocking
                // User can use refresh_provider_models_with_waf if they encounter WAF
            }
            Err(e) => {
                log::warn!("Failed to check WAF cookies cache: {}", e);
            }
        }
    }

    // Create token client and fetch models
    let client = TokenClient::new().map_err(CommandError::from)?;

    // Build cookie string from merged cookies
    let cookie_string: String = cookies
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("; ");

    // Get api_user from account credentials
    let api_user = account.credentials().api_user();

    // Try to fetch models
    let models_result = client
        .fetch_provider_models(
            &base_url,
            &models_path,
            &cookie_string,
            api_user_header_opt,
            Some(api_user),
        )
        .await;

    // Check if we got WAF challenge error
    let models = match models_result {
        Ok(models) => models,
        Err(e) => {
            // Check if it's a WAF challenge error
            let error_msg = e.to_string();
            if error_msg.contains("WAF_CHALLENGE:") {
                log::warn!("WAF challenge detected despite cached cookies, deleting cache");

                // Delete cached WAF cookies
                if let Err(inv_err) = repositories.waf_cookies.delete(&provider_id).await {
                    log::warn!("Failed to delete WAF cookies cache: {}", inv_err);
                }

                // Return helpful error message
                return Err(CommandError::infrastructure(
                    "WAF challenge detected. Please use 'Refresh with WAF' to refresh cookies.",
                ));
            }

            // Other error, just propagate
            return Err(CommandError::infrastructure(error_msg));
        }
    };

    // Cache the models
    repositories
        .provider_models
        .save(&provider_id, &models)
        .await
        .map_err(CommandError::from)?;

    log::info!(
        "Fetched and cached {} models for provider {}",
        models.len(),
        provider_id
    );

    Ok(models)
}
