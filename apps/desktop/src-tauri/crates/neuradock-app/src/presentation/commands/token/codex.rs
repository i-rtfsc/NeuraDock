use crate::presentation::error::CommandError;
use crate::presentation::state::{Repositories, Services};
use neuradock_domain::shared::{AccountId, ProviderId};
use tauri::State;

#[tauri::command]
#[specta::specta]
pub async fn configure_codex_global(
    token_id: i64,
    account_id: String,
    provider_id: String,
    base_url: String,
    model: Option<String>,
    services: State<'_, Services>,
    repositories: State<'_, Repositories>,
) -> Result<String, CommandError> {
    let account_id = AccountId::from_string(&account_id);
    let token_id = neuradock_domain::token::TokenId::new(token_id);

    // Get token from cache
    let tokens = services
        .token
        .get_cached_tokens(&account_id)
        .await
        .map_err(CommandError::from)?;

    let token = tokens
        .iter()
        .find(|t| t.id() == &token_id)
        .ok_or_else(|| CommandError::not_found("Token not found"))?;

    let provider_id_obj = ProviderId::from_string(&provider_id);
    let provider = repositories
        .provider
        .find_by_id(&provider_id_obj)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found(format!("Provider not found: {}", provider_id)))?;

    // Configure to Codex
    let result = services
        .codex_config
        .configure_global(
            token,
            provider.id().as_str(),
            provider.name(),
            &base_url,
            model.as_deref(),
        )
        .map_err(CommandError::from)?;

    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn generate_codex_temp_commands(
    token_id: i64,
    account_id: String,
    provider_id: String,
    base_url: String,
    model: Option<String>,
    services: State<'_, Services>,
    repositories: State<'_, Repositories>,
) -> Result<String, CommandError> {
    let account_id = AccountId::from_string(&account_id);
    let token_id = neuradock_domain::token::TokenId::new(token_id);

    // Get token from cache
    let tokens = services
        .token
        .get_cached_tokens(&account_id)
        .await
        .map_err(CommandError::from)?;

    let token = tokens
        .iter()
        .find(|t| t.id() == &token_id)
        .ok_or_else(|| CommandError::not_found("Token not found"))?;

    let provider_id_obj = ProviderId::from_string(&provider_id);
    let provider = repositories
        .provider
        .find_by_id(&provider_id_obj)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found(format!("Provider not found: {}", provider_id)))?;

    // Generate temp commands
    let commands = services
        .codex_config
        .generate_temp_commands(
            token,
            provider.id().as_str(),
            provider.name(),
            &base_url,
            model.as_deref(),
        )
        .map_err(CommandError::from)?;

    Ok(commands)
}

#[tauri::command]
#[specta::specta]
pub async fn clear_codex_global(services: State<'_, Services>) -> Result<String, CommandError> {
    services
        .codex_config
        .clear_global()
        .map_err(CommandError::from)
}
