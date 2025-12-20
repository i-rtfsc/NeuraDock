use crate::application::commands::command_handler::CommandHandler;
use crate::application::commands::notification_commands::*;
use crate::application::dtos::{
    CreateNotificationChannelInput, NotificationChannelDto, UpdateNotificationChannelInput,
};
use crate::presentation::error::CommandError;
use crate::presentation::state::{CommandHandlers, Repositories};
use tauri::State;

/// Create a notification channel
#[tauri::command]
#[specta::specta]
pub async fn create_notification_channel(
    input: CreateNotificationChannelInput,
    handlers: State<'_, CommandHandlers>,
) -> Result<NotificationChannelDto, CommandError> {
    let command = CreateNotificationChannelCommand { input };

    handlers
        .create_notification_channel
        .handle(command)
        .await
        .map_err(CommandError::from)
}

/// Update a notification channel
#[tauri::command]
#[specta::specta]
pub async fn update_notification_channel(
    input: UpdateNotificationChannelInput,
    handlers: State<'_, CommandHandlers>,
) -> Result<NotificationChannelDto, CommandError> {
    let command = UpdateNotificationChannelCommand { input };

    handlers
        .update_notification_channel
        .handle(command)
        .await
        .map_err(CommandError::from)
}

/// Delete a notification channel
#[tauri::command]
#[specta::specta]
pub async fn delete_notification_channel(
    channel_id: String,
    handlers: State<'_, CommandHandlers>,
) -> Result<(), CommandError> {
    let command = DeleteNotificationChannelCommand { channel_id };

    handlers
        .delete_notification_channel
        .handle(command)
        .await
        .map_err(CommandError::from)
}

/// Get all notification channels
#[tauri::command]
#[specta::specta]
pub async fn get_all_notification_channels(
    repositories: State<'_, Repositories>,
) -> Result<Vec<NotificationChannelDto>, CommandError> {
    let channels = repositories
        .notification_channel
        .find_all()
        .await
        .map_err(CommandError::from)?;

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
    handlers: State<'_, CommandHandlers>,
) -> Result<TestNotificationChannelResult, CommandError> {
    let command = TestNotificationChannelCommand { channel_id };

    handlers
        .test_notification_channel
        .handle(command)
        .await
        .map_err(CommandError::from)
}
