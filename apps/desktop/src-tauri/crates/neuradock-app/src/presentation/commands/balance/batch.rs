use crate::application::dtos::BalanceDto;
use crate::presentation::state::AppState;
use std::collections::HashMap;
use tauri::State;

use super::fetch::fetch_account_balance;

/// Fetch balances for multiple accounts
#[tauri::command]
#[specta::specta]
pub async fn fetch_accounts_balances(
    account_ids: Vec<String>,
    force_refresh: Option<bool>,
    state: State<'_, AppState>,
) -> Result<HashMap<String, Option<BalanceDto>>, String> {
    let mut results = HashMap::new();

    for account_id in account_ids {
        match fetch_account_balance(account_id.clone(), force_refresh, state.clone()).await {
            Ok(balance) => {
                results.insert(account_id, Some(balance));
            }
            Err(_) => {
                results.insert(account_id, None);
            }
        }
    }

    Ok(results)
}
