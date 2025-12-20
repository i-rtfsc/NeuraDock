use crate::application::dtos;
use crate::presentation::error::CommandError;
use crate::presentation::state::{Queries, Repositories};
use neuradock_domain::check_in::Provider;
use neuradock_domain::shared::AccountId;
use std::collections::HashMap;
use tauri::State;

/// Get all accounts (optionally filter by enabled status)
#[tauri::command]
#[specta::specta]
pub async fn get_all_accounts(
    enabled_only: bool,
    repositories: State<'_, Repositories>,
    queries: State<'_, Queries>,
) -> Result<Vec<dtos::AccountDto>, CommandError> {
    let providers = provider_map(&repositories).await.map_err(CommandError::from)?;

    queries
        .account
        .get_all_accounts(enabled_only, &providers)
        .await
        .map_err(CommandError::from)
}

/// Get account detail by ID
#[tauri::command]
#[specta::specta]
pub async fn get_account_detail(
    account_id: String,
    repositories: State<'_, Repositories>,
) -> Result<dtos::AccountDetailDto, CommandError> {
    let id = AccountId::from_string(&account_id);
    let account = repositories
        .account
        .find_by_id(&id)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found(format!("Account not found: {}", account_id)))?;

    use crate::application::dtos::AccountDetailDtoMapper;

    let providers = provider_map(&repositories).await.map_err(CommandError::from)?;
    let provider_name = providers
        .get(account.provider_id().as_str())
        .map(|p| p.name().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    Ok(AccountDetailDtoMapper::new(&account, provider_name)
        .with_balance(None)
        .to_dto())
}

async fn provider_map(
    repositories: &Repositories,
) -> Result<HashMap<String, Provider>, neuradock_domain::shared::DomainError> {
    let providers = repositories.provider.find_all().await?;
    Ok(providers
        .into_iter()
        .map(|provider| (provider.id().as_str().to_string(), provider))
        .collect())
}
