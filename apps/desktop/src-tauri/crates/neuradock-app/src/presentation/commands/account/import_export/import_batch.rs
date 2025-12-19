use crate::application::dtos::{BatchImportResult, ImportAccountInput, ImportItemResult};
use crate::presentation::state::AppState;
use tauri::State;
use tracing::warn;

use super::super::super::balance::fetch_account_balance;
use super::helpers::import_single_account;

/// Import multiple accounts from JSON (batch)
#[tauri::command]
#[specta::specta]
pub async fn import_accounts_batch(
    json_data: String,
    state: State<'_, AppState>,
) -> Result<BatchImportResult, String> {
    let inputs: Vec<ImportAccountInput> =
        serde_json::from_str(&json_data).map_err(|e| format!("Invalid JSON: {}", e))?;

    let mut results = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;

    for input in inputs {
        let account_name = input.name.clone();
        match import_single_account(input, &state.account_repo, &state.session_repo).await {
            Ok(account_id) => {
                succeeded += 1;
                if let Err(err) =
                    fetch_account_balance(account_id.clone(), Some(true), state.clone()).await
                {
                    warn!(
                        target: "neuradock::import",
                        account_id = %account_id,
                        "Failed to prefetch balance after batch import: {}",
                        err
                    );
                }
                results.push(ImportItemResult {
                    success: true,
                    account_id: Some(account_id),
                    account_name,
                    error: None,
                });
            }
            Err(e) => {
                failed += 1;
                results.push(ImportItemResult {
                    success: false,
                    account_id: None,
                    account_name,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(BatchImportResult {
        total: results.len() as i32,
        succeeded,
        failed,
        results,
    })
}
