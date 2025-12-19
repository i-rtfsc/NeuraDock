use crate::application::dtos::{
    BatchImportResult, BatchUpdateResult, ExportAccountsInput, ImportAccountInput,
    ImportItemResult, UpdateItemResult,
};
use crate::presentation::state::AppState;
use chrono::{Duration, Utc};
use neuradock_domain::account::{Account, Credentials};
use neuradock_domain::session::{Session, SessionRepository};
use neuradock_domain::shared::{AccountId, ProviderId};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;
use tracing::warn;

use super::super::balance::fetch_account_balance;

const DEFAULT_SESSION_EXPIRATION_DAYS: i64 = 30;

/// Helper function to create and save a default session for an account
async fn create_and_save_default_session(
    account_id: AccountId,
    cookies: &HashMap<String, String>,
    session_repo: &Arc<dyn SessionRepository>,
) -> Result<(), String> {
    let session_token = cookies
        .values()
        .next()
        .cloned()
        .unwrap_or_else(|| "session".to_string());

    let expires_at = Utc::now() + Duration::days(DEFAULT_SESSION_EXPIRATION_DAYS);
    let session = Session::new(account_id, session_token, expires_at).map_err(|e| e.to_string())?;

    session_repo.save(&session).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Helper function to import a single account
async fn import_single_account(
    input: ImportAccountInput,
    account_repo: &Arc<dyn crate::domain::account::AccountRepository>,
    session_repo: &Arc<dyn SessionRepository>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let cookies = input.cookies.clone();
    let credentials = Credentials::new(input.cookies, input.api_user);
    let account = Account::new(
        input.name,
        ProviderId::from_string(&input.provider),
        credentials,
    )?;

    let account_id = account.id().clone();
    account_repo.save(&account).await?;

    create_and_save_default_session(account_id.clone(), &cookies, session_repo).await?;

    Ok(account_id.as_str().to_string())
}

/// Helper function to update account cookies
async fn update_account_cookies(
    account_id: &AccountId,
    cookies: HashMap<String, String>,
    api_user: String,
    account_repo: &Arc<dyn crate::domain::account::AccountRepository>,
    session_repo: &Arc<dyn SessionRepository>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut account = account_repo
        .find_by_id(account_id)
        .await?
        .ok_or("Account not found")?;

    let credentials = Credentials::new(cookies.clone(), api_user);
    account.update_credentials(credentials)?;
    account_repo.save(&account).await?;

    create_and_save_default_session(account_id.clone(), &cookies, session_repo).await?;

    Ok(())
}

/// Import a single account from JSON
#[tauri::command]
#[specta::specta]
pub async fn import_account_from_json(
    json_data: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let input: ImportAccountInput =
        serde_json::from_str(&json_data).map_err(|e| format!("Invalid JSON: {}", e))?;

    let cookies = input.cookies.clone();
    let credentials = Credentials::new(input.cookies, input.api_user);
    let account = Account::new(
        input.name,
        ProviderId::from_string(&input.provider),
        credentials,
    )
    .map_err(|e| e.to_string())?;

    let account_id = account.id().clone();

    state
        .account_repo
        .save(&account)
        .await
        .map_err(|e| e.to_string())?;

    create_and_save_default_session(account_id.clone(), &cookies, &state.session_repo).await?;

    let account_id_str = account_id.as_str().to_string();
    if let Err(err) = fetch_account_balance(account_id_str.clone(), Some(true), state.clone()).await
    {
        warn!(
            target: "neuradock::import",
            account_id = %account_id_str,
            "Failed to prefetch balance after import: {}",
            err
        );
    }

    Ok(account_id_str)
}

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
        succeeded: succeeded as i32,
        failed: failed as i32,
        results,
    })
}

/// Export accounts to JSON
#[tauri::command]
#[specta::specta]
pub async fn export_accounts_to_json(
    input: ExportAccountsInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let accounts = if input.account_ids.is_empty() {
        state
            .account_repo
            .find_all()
            .await
            .map_err(|e| e.to_string())?
    } else {
        let mut result = Vec::new();
        for id_str in input.account_ids {
            let id = AccountId::from_string(&id_str);
            if let Some(account) = state
                .account_repo
                .find_by_id(&id)
                .await
                .map_err(|e| e.to_string())?
            {
                result.push(account);
            }
        }
        result
    };

    let export_data: Vec<serde_json::Value> = accounts
        .iter()
        .map(|acc| {
            let mut data = serde_json::json!({
                "name": acc.name(),
                "provider": acc.provider_id().as_str(),
            });

            if input.include_credentials {
                data["cookies"] = serde_json::to_value(acc.credentials().cookies())
                    .expect("Failed to serialize cookies");
                data["api_user"] =
                    serde_json::Value::String(acc.credentials().api_user().to_string());
            }

            data
        })
        .collect();

    serde_json::to_string_pretty(&export_data).map_err(|e| e.to_string())
}

/// Batch update accounts - match by name+provider, update cookies and api_user
/// If account doesn't exist and create_if_not_exists is true, create it
#[tauri::command]
#[specta::specta]
pub async fn update_accounts_batch(
    json_data: String,
    create_if_not_exists: bool,
    state: State<'_, AppState>,
) -> Result<BatchUpdateResult, String> {
    let inputs: Vec<ImportAccountInput> =
        serde_json::from_str(&json_data).map_err(|e| format!("Invalid JSON: {}", e))?;

    // Load all existing accounts for matching
    let existing_accounts = state
        .account_repo
        .find_all()
        .await
        .map_err(|e| e.to_string())?;

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
        updated: updated as i32,
        created: created as i32,
        failed: failed as i32,
        results,
    })
}
