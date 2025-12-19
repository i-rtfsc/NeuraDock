use crate::application::ResultExt;
use crate::presentation::state::AppState;
use neuradock_domain::account::{Account, Credentials};
use neuradock_domain::shared::ProviderId;
use tauri::State;
use tracing::warn;

use super::super::super::balance::fetch_account_balance;
use super::helpers::create_and_save_default_session;

/// Import a single account from JSON
#[tauri::command]
#[specta::specta]
pub async fn import_account_from_json(
    json_data: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let input: crate::application::dtos::ImportAccountInput =
        serde_json::from_str(&json_data).map_err(|e| format!("Invalid JSON: {}", e))?;

    let cookies = input.cookies.clone();
    let credentials = Credentials::new(input.cookies, input.api_user);
    let account = Account::new(
        input.name,
        ProviderId::from_string(&input.provider),
        credentials,
    )
    .to_string_err()?;

    let account_id = account.id().clone();

    state
        .account_repo
        .save(&account)
        .await
        .to_string_err()?;

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
