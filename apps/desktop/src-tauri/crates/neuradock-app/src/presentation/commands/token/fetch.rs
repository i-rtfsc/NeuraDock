use crate::application::dtos::TokenDto;
use crate::presentation::error::CommandError;
use crate::presentation::state::{Repositories, Services};
use neuradock_domain::shared::AccountId;
use tauri::State;

#[tauri::command]
#[specta::specta]
pub async fn fetch_account_tokens(
    account_id: String,
    force_refresh: bool,
    services: State<'_, Services>,
    repositories: State<'_, Repositories>,
) -> Result<Vec<TokenDto>, CommandError> {
    log::info!(
        "fetch_account_tokens called: account_id={}, force_refresh={}",
        account_id,
        force_refresh
    );
    let account_id = AccountId::from_string(&account_id);

    // Fetch tokens from service
    let tokens = services
        .token
        .fetch_and_cache_tokens(&account_id, force_refresh)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch tokens: {}", e);
            CommandError::from(e)
        })?;

    log::info!("Fetched {} tokens for account {}", tokens.len(), account_id);

    // Get account info to fill DTO
    let account = repositories
        .account
        .find_by_id(&account_id)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found("Account not found"))?;

    // Get provider info
    let provider_id = account.provider_id().as_str().to_string();
    let provider = repositories
        .provider
        .find_by_id(account.provider_id())
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found(format!("Provider not found: {}", provider_id)))?;

    // Convert to DTOs
    let dtos = tokens
        .iter()
        .map(|token| {
            TokenDto::from_domain(
                token,
                account.name().to_string(),
                provider.name().to_string(),
            )
        })
        .collect();

    Ok(dtos)
}
