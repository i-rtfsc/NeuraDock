use crate::application::commands::account_commands::*;
use crate::application::commands::command_handler::CommandHandler;
use crate::application::dtos::CreateAccountInput;
use crate::application::dtos::UpdateAccountInput;
use crate::presentation::error::CommandError;
use crate::presentation::state::CommandHandlers;
use tauri::State;

/// Create a new account
#[tauri::command]
#[specta::specta]
pub async fn create_account(
    input: CreateAccountInput,
    state: State<'_, CommandHandlers>,
) -> Result<String, CommandError> {
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
        .create_account
        .handle(command)
        .await
        .map_err(CommandError::from)?;

    // Scheduler will be reloaded automatically via AccountCreated event
    // handled by SchedulerReloadEventHandler

    Ok(result.account_id)
}

/// Update an existing account
#[tauri::command]
#[specta::specta]
pub async fn update_account(
    input: UpdateAccountInput,
    state: State<'_, CommandHandlers>,
) -> Result<bool, CommandError> {
    let command = UpdateAccountCommand {
        account_id: input.account_id,
        name: input.name,
        cookies: input.cookies,
        api_user: input.api_user,
        auto_checkin_enabled: input.auto_checkin_enabled,
        auto_checkin_hour: input.auto_checkin_hour,
        auto_checkin_minute: input.auto_checkin_minute,
        check_in_interval_hours: input.check_in_interval_hours,
    };

    let result = state
        .update_account
        .handle(command)
        .await
        .map_err(CommandError::from)?;

    // Scheduler will be reloaded automatically via AccountUpdated event
    // handled by SchedulerReloadEventHandler

    Ok(result.success)
}

/// Delete an account
#[tauri::command]
#[specta::specta]
pub async fn delete_account(
    account_id: String,
    state: State<'_, CommandHandlers>,
) -> Result<bool, CommandError> {
    let command = DeleteAccountCommand { account_id };

    let result = state
        .delete_account
        .handle(command)
        .await
        .map_err(CommandError::from)?;

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
    state: State<'_, CommandHandlers>,
) -> Result<bool, CommandError> {
    let command = ToggleAccountCommand {
        account_id,
        enabled,
    };

    let result = state
        .toggle_account
        .handle(command)
        .await
        .map_err(CommandError::from)?;

    // Scheduler will be reloaded automatically via AccountToggled event
    // handled by SchedulerReloadEventHandler

    Ok(result.success)
}
