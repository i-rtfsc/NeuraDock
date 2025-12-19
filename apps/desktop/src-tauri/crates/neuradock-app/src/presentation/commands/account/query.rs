use crate::application::dtos;
use crate::presentation::state::AppState;
use neuradock_domain::shared::AccountId;
use tauri::State;

/// Get all accounts (optionally filter by enabled status)
#[tauri::command]
#[specta::specta]
pub async fn get_all_accounts(
    enabled_only: bool,
    state: State<'_, AppState>,
) -> Result<Vec<dtos::AccountDto>, String> {
    let providers = state.provider_map().await.map_err(|e| e.to_string())?;

    state
        .account_queries
        .get_all_accounts(enabled_only, &providers)
        .await
        .map_err(|e| e.to_string())
}

/// Get account detail by ID
#[tauri::command]
#[specta::specta]
pub async fn get_account_detail(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<dtos::AccountDetailDto, String> {
    let id = AccountId::from_string(&account_id);
    let account = state
        .account_repo
        .find_by_id(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Account not found")?;

    use crate::application::dtos::AccountDetailDtoMapper;

    let providers = state.provider_map().await.map_err(|e| e.to_string())?;
    let provider_name = providers
        .get(account.provider_id().as_str())
        .map(|p| p.name().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    Ok(AccountDetailDtoMapper::new(&account, provider_name)
        .with_balance(None)
        .to_dto())
}
