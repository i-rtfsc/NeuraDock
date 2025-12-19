use crate::application::dtos::ExportAccountsInput;
use crate::application::ResultExt;
use crate::presentation::state::AppState;
use neuradock_domain::shared::AccountId;
use tauri::State;

/// Export accounts to JSON
#[tauri::command]
#[specta::specta]
pub async fn export_accounts_to_json(
    input: ExportAccountsInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let accounts = if input.account_ids.is_empty() {
        state.account_repo.find_all().await.to_string_err()?
    } else {
        let mut result = Vec::new();
        for id_str in input.account_ids {
            let id = AccountId::from_string(&id_str);
            if let Some(account) = state
                .account_repo
                .find_by_id(&id)
                .await
                .to_string_err()?
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

    serde_json::to_string_pretty(&export_data).to_string_err()
}
