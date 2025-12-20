use crate::application::dtos::BalanceDto;
use crate::presentation::error::CommandError;
use crate::presentation::state::Services;
use tauri::State;

/// Fetch account balance with smart caching
/// Only performs fresh login if cache is stale (> 1 hour) or no cache exists
/// Set force_refresh to true to ignore cache and always fetch fresh balance
#[tauri::command]
#[specta::specta]
pub async fn fetch_account_balance(
    account_id: String,
    force_refresh: Option<bool>,
    state: State<'_, Services>,
) -> Result<BalanceDto, CommandError> {
    let force_refresh = force_refresh.unwrap_or(false);
    state
        .balance
        .fetch_account_balance(&account_id, force_refresh)
        .await
        .map_err(CommandError::from)
}
