use crate::application::dtos::{BatchImportResult, ImportAccountInput, ImportItemResult};
use crate::presentation::error::CommandError;
use crate::presentation::state::{Repositories, Services};
use tauri::State;
use tracing::warn;

use super::helpers::import_single_account;

/// Import multiple accounts from JSON (batch)
#[tauri::command]
#[specta::specta]
pub async fn import_accounts_batch(
    json_data: String,
    repositories: State<'_, Repositories>,
    services: State<'_, Services>,
) -> Result<BatchImportResult, CommandError> {
    let inputs: Vec<ImportAccountInput> =
        serde_json::from_str(&json_data).map_err(CommandError::from)?;

    let mut results = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;

    for input in inputs {
        let account_name = input.name.clone();
        match import_single_account(
            input,
            &repositories.account,
            &repositories.session,
        )
        .await
        {
            Ok(account_id) => {
                succeeded += 1;
                if let Err(err) = services
                    .balance
                    .fetch_account_balance(&account_id, true)
                    .await
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
