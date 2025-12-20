use crate::application::dtos::{BatchUpdateResult, ImportAccountInput, UpdateItemResult};
use crate::presentation::error::CommandError;
use crate::presentation::state::AppState;
use tauri::State;

use super::helpers::{import_single_account, update_account_cookies};

/// Batch update accounts - match by name+provider, update cookies and api_user
/// If account doesn't exist and create_if_not_exists is true, create it
#[tauri::command]
#[specta::specta]
pub async fn update_accounts_batch(
    json_data: String,
    create_if_not_exists: bool,
    state: State<'_, AppState>,
) -> Result<BatchUpdateResult, CommandError> {
    let inputs: Vec<ImportAccountInput> =
        serde_json::from_str(&json_data).map_err(CommandError::from)?;

    // Load all existing accounts for matching
    let existing_accounts = state
        .account_repo
        .find_all()
        .await
        .map_err(CommandError::from)?;

    let mut results = Vec::new();
    let mut updated = 0;
    let mut created = 0;
    let mut failed = 0;

    for input in inputs {
        let account_name = input.name.clone();
        let provider_id = input.provider.clone();

        // Try to find existing account by name + provider
        let existing = existing_accounts
            .iter()
            .find(|acc| acc.name() == input.name && acc.provider_id().as_str() == input.provider);

        match existing {
            Some(acc) => {
                // Update existing account
                let account_id = acc.id().clone();
                match update_account_cookies(
                    &account_id,
                    input.cookies,
                    input.api_user,
                    &state.account_repo,
                    &state.session_repo,
                )
                .await
                {
                    Ok(_) => {
                        updated += 1;
                        results.push(UpdateItemResult {
                            success: true,
                            account_id: Some(account_id.as_str().to_string()),
                            account_name,
                            action: "updated".to_string(),
                            error: None,
                        });
                    }
                    Err(e) => {
                        failed += 1;
                        results.push(UpdateItemResult {
                            success: false,
                            account_id: Some(account_id.as_str().to_string()),
                            account_name,
                            action: "failed".to_string(),
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
            None => {
                if create_if_not_exists {
                    // Create new account
                    match import_single_account(input, &state.account_repo, &state.session_repo)
                        .await
                    {
                        Ok(account_id) => {
                            created += 1;
                            results.push(UpdateItemResult {
                                success: true,
                                account_id: Some(account_id),
                                account_name,
                                action: "created".to_string(),
                                error: None,
                            });
                        }
                        Err(e) => {
                            failed += 1;
                            results.push(UpdateItemResult {
                                success: false,
                                account_id: None,
                                account_name,
                                action: "failed".to_string(),
                                error: Some(e.to_string()),
                            });
                        }
                    }
                } else {
                    failed += 1;
                    results.push(UpdateItemResult {
                        success: false,
                        account_id: None,
                        account_name,
                        action: "failed".to_string(),
                        error: Some(format!("Account not found (provider: {})", provider_id)),
                    });
                }
            }
        }
    }

    // Scheduler will be reloaded automatically via domain events
    // (AccountCreated/AccountUpdated) handled by SchedulerReloadEventHandler

    Ok(BatchUpdateResult {
        total: results.len() as i32,
        updated,
        created,
        failed,
        results,
    })
}
