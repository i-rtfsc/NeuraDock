use crate::application::dtos::ExportAccountsInput;
use crate::presentation::error::CommandError;
use crate::presentation::state::AppState;
use neuradock_domain::shared::AccountId;
use tauri::State;

/// Export accounts to JSON
#[tauri::command]
#[specta::specta]
pub async fn export_accounts_to_json(
    input: ExportAccountsInput,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    let accounts = if input.account_ids.is_empty() {
        state
            .account_repo
            .find_all()
            .await
            .map_err(CommandError::from)?
    } else {
        let ids: Vec<AccountId> = input
            .account_ids
            .iter()
            .map(|id| AccountId::from_string(id))
            .collect();
        state
            .account_repo
            .find_by_ids(&ids)
            .await
            .map_err(CommandError::from)?
    };

    let export_data = accounts
        .iter()
        .map(|acc| -> Result<serde_json::Value, CommandError> {
            let mut data = serde_json::json!({
                "name": acc.name(),
                "provider": acc.provider_id().as_str(),
            });

            if input.include_credentials {
                data["cookies"] =
                    serde_json::to_value(acc.credentials().cookies()).map_err(CommandError::from)?;
                data["api_user"] =
                    serde_json::Value::String(acc.credentials().api_user().to_string());
            }

            Ok(data)
        })
        .collect::<Result<Vec<_>, _>>()?;

    serde_json::to_string_pretty(&export_data).map_err(CommandError::from)
}
