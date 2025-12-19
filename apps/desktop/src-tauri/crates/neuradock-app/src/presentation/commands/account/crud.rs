use crate::application::commands::account_commands::*;
use crate::application::commands::command_handler::CommandHandler;
use crate::application::dtos::CreateAccountInput;
use crate::application::dtos::UpdateAccountInput;
use crate::presentation::state::AppState;
use tauri::State;

/// Create a new account
#[tauri::command]
#[specta::specta]
pub async fn create_account(
    input: CreateAccountInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let command = CreateAccountCommand {
        name: input.name,
        provider_id: input.provider_id,
        cookies: input.cookies,
        api_user: input.api_user,
        auto_checkin_enabled: input.auto_checkin_enabled,
        auto_checkin_hour: input.auto_checkin_hour,
        auto_checkin_minute: input.auto_checkin_minute,
    };

    let result = state
        .command_handlers
        .create_account
        .handle(command)
        .await
        .map_err(|e| e.to_string())?;

    // Scheduler will be reloaded automatically via AccountCreated event
    // handled by SchedulerReloadEventHandler

    Ok(result.account_id)
}

/// Update an existing account
#[tauri::command]
#[specta::specta]
pub async fn update_account(
    input: UpdateAccountInput,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let command = UpdateAccountCommand {
        account_id: input.account_id,
        name: input.name,
        cookies: input.cookies,
        api_user: input.api_user,
        auto_checkin_enabled: input.auto_checkin_enabled,
        auto_checkin_hour: input.auto_checkin_hour,
        auto_checkin_minute: input.auto_checkin_minute,
    };

    let result = state
        .command_handlers
        .update_account
        .handle(command)
        .await
        .map_err(|e| e.to_string())?;

    // Scheduler will be reloaded automatically via AccountUpdated event
    // handled by SchedulerReloadEventHandler

    Ok(result.success)
}

/// Delete an account
#[tauri::command]
#[specta::specta]
pub async fn delete_account(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let command = DeleteAccountCommand { account_id };

    let result = state
        .command_handlers
        .delete_account
        .handle(command)
        .await
        .map_err(|e| e.to_string())?;

    // Scheduler will be reloaded automatically via AccountDeleted event
    // handled by SchedulerReloadEventHandler

    Ok(result.success)
}

/// Toggle account enabled/disabled status
#[tauri::command]
#[specta::specta]
pub async fn toggle_account(
    account_id: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let command = ToggleAccountCommand {
        account_id,
        enabled,
    };

    let result = state
        .command_handlers
        .toggle_account
        .handle(command)
        .await
        .map_err(|e| e.to_string())?;

    // Scheduler will be reloaded automatically via AccountToggled event
    // handled by SchedulerReloadEventHandler

    Ok(result.success)
}
