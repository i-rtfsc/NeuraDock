use tauri::State;

use crate::application::dtos::{ReminderSettingsDto, UpdateReminderSettingsInput};
use crate::presentation::error::CommandError;
use crate::presentation::state::Services;

/// Get current reminder settings
#[tauri::command]
#[specta::specta]
pub async fn get_reminder_settings(
    state: State<'_, Services>,
) -> Result<ReminderSettingsDto, CommandError> {
    state
        .reminder_settings
        .get()
        .await
        .map_err(CommandError::from)
}

/// Update reminder settings
#[tauri::command]
#[specta::specta]
pub async fn update_reminder_settings(
    input: UpdateReminderSettingsInput,
    state: State<'_, Services>,
) -> Result<ReminderSettingsDto, CommandError> {
    state
        .reminder_settings
        .update(input)
        .await
        .map_err(CommandError::from)
}
