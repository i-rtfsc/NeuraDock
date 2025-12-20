use crate::presentation::error::CommandError;
use crate::presentation::state::{Repositories, Services};
use neuradock_domain::account::{Account, Credentials};
use neuradock_domain::shared::ProviderId;
use tauri::State;
use tracing::warn;

use super::helpers::create_and_save_default_session;

/// Import a single account from JSON
#[tauri::command]
#[specta::specta]
pub async fn import_account_from_json(
    json_data: String,
    repositories: State<'_, Repositories>,
    services: State<'_, Services>,
) -> Result<String, CommandError> {
    let input: crate::application::dtos::ImportAccountInput =
        serde_json::from_str(&json_data).map_err(CommandError::from)?;

    let cookies = input.cookies.clone();
    let credentials = Credentials::new(input.cookies, input.api_user);
    let account = Account::new(
        input.name,
        ProviderId::from_string(&input.provider),
        credentials,
    )
    .map_err(CommandError::from)?;

    let account_id = account.id().clone();

    repositories
        .account
        .save(&account)
        .await
        .map_err(CommandError::from)?;

    create_and_save_default_session(account_id.clone(), &cookies, &repositories.session)
        .await?;

    let account_id_str = account_id.as_str().to_string();
    if let Err(err) = services
        .balance
        .fetch_account_balance(&account_id_str, true)
        .await
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
