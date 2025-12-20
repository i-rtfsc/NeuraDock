use crate::presentation::error::CommandError;
use crate::presentation::state::AppState;
use neuradock_domain::shared::AccountId;
use tauri::State;
/// Refresh provider models with WAF bypass
/// This command will:
/// 1. Check for cached WAF cookies (valid for 24 hours)
/// 2. If cached cookies are expired or missing, run WAF bypass
/// 3. Use cookies to fetch model list
/// 4. Save models and WAF cookies to database
#[tauri::command]
#[specta::specta]
pub async fn refresh_provider_models_with_waf(
    provider_id: String,
    account_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, CommandError> {
    use neuradock_infrastructure::http::{token::TokenClient, WafBypassService};

    log::info!(
        "refresh_provider_models_with_waf: provider_id={}, account_id={}",
        provider_id,
        account_id
    );

    // Get account
    let account_id_obj = AccountId::from_string(&account_id);
    let account = state
        .account_repo
        .find_by_id(&account_id_obj)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found("Account not found"))?;

    // Get provider configuration
    let provider_id_obj = neuradock_domain::shared::ProviderId::from_string(&provider_id);
    let provider = state
        .provider_repo
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

    // Handle WAF bypass with caching
    if provider.needs_waf_bypass() {
        // Check for cached WAF cookies first
        match state.waf_cookies_repo.get_valid(&provider_id).await {
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
                // No valid cached cookies, run WAF bypass
                log::info!(
                    "No valid cached WAF cookies, running WAF bypass for provider {}",
                    provider_id
                );

                let waf_service = WafBypassService::new(true); // headless mode
                match waf_service
                    .get_waf_cookies(&provider.login_url(), &account_id)
                    .await
                {
                    Ok(new_cookies) => {
                        log::info!("WAF bypass successful, got {} cookies", new_cookies.len());

                        // Save WAF cookies to database for future use
                        if let Err(e) = state
                            .waf_cookies_repo
                            .save(&provider_id, &new_cookies)
                            .await
                        {
                            log::warn!("Failed to cache WAF cookies: {}", e);
                        }

                        // Merge new cookies
                        for (k, v) in new_cookies {
                            cookies.insert(k, v);
                        }
                    }
                    Err(e) => {
                        log::error!("WAF bypass failed: {}", e);
                        return Err(CommandError::infrastructure(format!(
                            "WAF bypass failed: {}",
                            e
                        )));
                    }
                }
            }
            Err(e) => {
                log::warn!(
                    "Failed to check cached WAF cookies: {}, proceeding with bypass",
                    e
                );
                // Fall back to running WAF bypass
                let waf_service = WafBypassService::new(true);
                match waf_service
                    .get_waf_cookies(&provider.login_url(), &account_id)
                    .await
                {
                    Ok(new_cookies) => {
                        if let Err(e) = state
                            .waf_cookies_repo
                            .save(&provider_id, &new_cookies)
                            .await
                        {
                            log::warn!("Failed to cache WAF cookies: {}", e);
                        }
                        for (k, v) in new_cookies {
                            cookies.insert(k, v);
                        }
                    }
                    Err(e) => {
                        return Err(CommandError::infrastructure(format!(
                            "WAF bypass failed: {}",
                            e
                        )));
                    }
                }
            }
        }
    }

    // Build cookie string
    let cookie_string: String = cookies
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("; ");

    // Get api_user from account credentials (not from provider config!)
    let api_user = account.credentials().api_user();

    // Fetch models - with retry on WAF challenge
    let client = TokenClient::new().map_err(CommandError::from)?;
    let models_result = client
        .fetch_provider_models(
            &base_url,
            &models_path,
            &cookie_string,
            api_user_header_opt,
            Some(api_user),
        )
        .await;

    // Check if we got WAF challenge - if so, retry with fresh cookies
    let models = match models_result {
        Ok(models) => models,
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("WAF_CHALLENGE:") {
                log::warn!("WAF challenge detected despite cached cookies! Invalidating cache and retrying with WAF bypass...");

                // Delete invalid cached cookies
                if let Err(del_err) = state.waf_cookies_repo.delete(&provider_id).await {
                    log::warn!("Failed to delete invalid WAF cookies: {}", del_err);
                }

                // Run WAF bypass to get fresh cookies
                use neuradock_infrastructure::http::WafBypassService;
                let waf_service = WafBypassService::new(true); // headless
                let fresh_waf_cookies = waf_service
                    .get_waf_cookies(&provider.login_url(), &account_id)
                    .await
                    .map_err(|e| CommandError::infrastructure(format!("WAF bypass failed: {}", e)))?;

                log::info!(
                    "WAF bypass successful, got {} fresh cookies",
                    fresh_waf_cookies.len()
                );

                // Save fresh cookies to cache
                if let Err(save_err) = state
                    .waf_cookies_repo
                    .save(&provider_id, &fresh_waf_cookies)
                    .await
                {
                    log::warn!("Failed to cache fresh WAF cookies: {}", save_err);
                }

                // Merge fresh WAF cookies with account cookies
                let mut fresh_cookies = account.credentials().cookies().clone();
                for (k, v) in fresh_waf_cookies {
                    fresh_cookies.insert(k, v);
                }

                // Build new cookie string
                let fresh_cookie_string: String = fresh_cookies
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join("; ");

                // Retry fetching models with fresh cookies
                log::info!("Retrying model fetch with fresh WAF cookies...");
                client
                    .fetch_provider_models(
                        &base_url,
                        &models_path,
                        &fresh_cookie_string,
                        api_user_header_opt,
                        Some(api_user),
                    )
                    .await
                    .map_err(|e| {
                        CommandError::infrastructure(format!(
                            "Failed to fetch models even after WAF bypass: {}",
                            e
                        ))
                    })?
            } else {
                // Not a WAF challenge, return original error
                return Err(CommandError::infrastructure(error_msg));
            }
        }
    };

    // Save to database
    state
        .provider_models_repo
        .save(&provider_id, &models)
        .await
        .map_err(CommandError::from)?;

    log::info!(
        "Refreshed and saved {} models for provider {}",
        models.len(),
        provider_id
    );

    Ok(models)
}
