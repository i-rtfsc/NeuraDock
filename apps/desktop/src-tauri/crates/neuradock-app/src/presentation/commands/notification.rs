use crate::application::commands::command_handler::CommandHandler;
use crate::application::commands::notification_commands::*;
use crate::application::dtos::{
    CreateNotificationChannelInput, NotificationChannelDto, UpdateNotificationChannelInput,
};
use crate::presentation::state::AppState;
use tauri::State;

/// Create a notification channel
#[tauri::command]
#[specta::specta]
pub async fn create_notification_channel(
    input: CreateNotificationChannelInput,
    state: State<'_, AppState>,
) -> Result<NotificationChannelDto, String> {
    let command = CreateNotificationChannelCommand { input };

    state
        .command_handlers
        .create_notification_channel
        .handle(command)
        .await
        .map_err(|e| e.to_string())
}

/// Update a notification channel
#[tauri::command]
#[specta::specta]
pub async fn update_notification_channel(
    input: UpdateNotificationChannelInput,
    state: State<'_, AppState>,
) -> Result<NotificationChannelDto, String> {
    let command = UpdateNotificationChannelCommand { input };

    state
        .command_handlers
        .update_notification_channel
        .handle(command)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a notification channel
#[tauri::command]
#[specta::specta]
pub async fn delete_notification_channel(
    channel_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let command = DeleteNotificationChannelCommand { channel_id };

    state
        .command_handlers
        .delete_notification_channel
        .handle(command)
        .await
        .map_err(|e| e.to_string())
}

/// Get all notification channels
#[tauri::command]
#[specta::specta]
pub async fn get_all_notification_channels(
    state: State<'_, AppState>,
) -> Result<Vec<NotificationChannelDto>, String> {
    let channels = state
        .notification_channel_repo
        .find_all()
        .await
        .map_err(|e| e.to_string())?;

    let dtos = channels
        .iter()
        .map(|channel| NotificationChannelDto {
            id: channel.id().as_str().to_string(),
            channel_type: channel.channel_type().as_str().to_string(),
            config: serde_json::to_value(channel.config()).unwrap_or(serde_json::json!({})),
            enabled: channel.is_enabled(),
            created_at: channel.created_at().to_rfc3339(),
        })
        .collect();

    Ok(dtos)
}

/// Test a notification channel
#[tauri::command]
#[specta::specta]
pub async fn test_notification_channel(
    channel_id: String,
    state: State<'_, AppState>,
) -> Result<TestNotificationChannelResult, String> {
    let command = TestNotificationChannelCommand { channel_id };

    state
        .command_handlers
        .test_notification_channel
        .handle(command)
        .await
        .map_err(|e| e.to_string())
}
