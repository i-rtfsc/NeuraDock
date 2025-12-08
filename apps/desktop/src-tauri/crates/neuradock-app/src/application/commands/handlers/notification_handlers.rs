use async_trait::async_trait;
use log::info;
use std::sync::Arc;

use crate::application::commands::command_handler::CommandHandler;
use crate::application::commands::notification_commands::*;
use crate::application::dtos::NotificationChannelDto;
use neuradock_domain::notification::{
    ChannelConfig, ChannelType, NotificationChannel, NotificationChannelId,
    NotificationChannelRepository,
};
use neuradock_domain::shared::DomainError;
use neuradock_infrastructure::notification::create_sender;

/// Create notification channel command handler
pub struct CreateNotificationChannelHandler {
    channel_repo: Arc<dyn NotificationChannelRepository>,
}

impl CreateNotificationChannelHandler {
    pub fn new(channel_repo: Arc<dyn NotificationChannelRepository>) -> Self {
        Self { channel_repo }
    }
}

#[async_trait]
impl CommandHandler<CreateNotificationChannelCommand> for CreateNotificationChannelHandler {
    type Result = CreateNotificationChannelResult;

    async fn handle(
        &self,
        cmd: CreateNotificationChannelCommand,
    ) -> Result<Self::Result, DomainError> {
        info!("Creating notification channel: {}", cmd.input.channel_type);

        // Parse channel type
        let channel_type = ChannelType::from_str(&cmd.input.channel_type)?;

        // Parse and validate config
        let config: ChannelConfig = serde_json::from_value(cmd.input.config)
            .map_err(|e| DomainError::InvalidInput(format!("Invalid channel config: {}", e)))?;

        // Validate channel type matches config
        if config.channel_type() != channel_type {
            return Err(DomainError::InvalidInput(
                "Channel type mismatch between type and config".to_string(),
            ));
        }

        // Create domain aggregate
        let channel = NotificationChannel::new(config)?;

        // Persist
        self.channel_repo.save(&channel).await?;

        info!("Notification channel created: {}", channel.id());

        // Return DTO
        Ok(NotificationChannelDto {
            id: channel.id().as_str().to_string(),
            channel_type: channel.channel_type().as_str().to_string(),
            config: serde_json::to_value(channel.config()).map_err(|e| {
                DomainError::Serialization(format!("Failed to serialize config: {}", e))
            })?,
            enabled: channel.is_enabled(),
            created_at: channel.created_at().to_rfc3339(),
        })
    }
}

/// Update notification channel command handler
pub struct UpdateNotificationChannelHandler {
    channel_repo: Arc<dyn NotificationChannelRepository>,
}

impl UpdateNotificationChannelHandler {
    pub fn new(channel_repo: Arc<dyn NotificationChannelRepository>) -> Self {
        Self { channel_repo }
    }
}

#[async_trait]
impl CommandHandler<UpdateNotificationChannelCommand> for UpdateNotificationChannelHandler {
    type Result = UpdateNotificationChannelResult;

    async fn handle(
        &self,
        cmd: UpdateNotificationChannelCommand,
    ) -> Result<Self::Result, DomainError> {
        info!("Updating notification channel: {}", cmd.input.channel_id);

        let channel_id = NotificationChannelId::from_string(&cmd.input.channel_id);

        // Load channel
        let mut channel = self
            .channel_repo
            .find_by_id(&channel_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("Channel not found: {}", cmd.input.channel_id))
            })?;

        // Update config if provided
        if let Some(config_value) = cmd.input.config {
            let config: ChannelConfig = serde_json::from_value(config_value)
                .map_err(|e| DomainError::InvalidInput(format!("Invalid channel config: {}", e)))?;
            channel.update_config(config)?;
        }

        // Update enabled status if provided
        if let Some(enabled) = cmd.input.enabled {
            if enabled {
                channel.enable();
            } else {
                channel.disable();
            }
        }

        // Persist
        self.channel_repo.update(&channel).await?;

        info!("Notification channel updated: {}", channel.id());

        // Return DTO
        Ok(NotificationChannelDto {
            id: channel.id().as_str().to_string(),
            channel_type: channel.channel_type().as_str().to_string(),
            config: serde_json::to_value(channel.config()).map_err(|e| {
                DomainError::Serialization(format!("Failed to serialize config: {}", e))
            })?,
            enabled: channel.is_enabled(),
            created_at: channel.created_at().to_rfc3339(),
        })
    }
}

/// Delete notification channel command handler
pub struct DeleteNotificationChannelHandler {
    channel_repo: Arc<dyn NotificationChannelRepository>,
}

impl DeleteNotificationChannelHandler {
    pub fn new(channel_repo: Arc<dyn NotificationChannelRepository>) -> Self {
        Self { channel_repo }
    }
}

#[async_trait]
impl CommandHandler<DeleteNotificationChannelCommand> for DeleteNotificationChannelHandler {
    type Result = DeleteNotificationChannelResult;

    async fn handle(
        &self,
        cmd: DeleteNotificationChannelCommand,
    ) -> Result<Self::Result, DomainError> {
        info!("Deleting notification channel: {}", cmd.channel_id);

        let channel_id = NotificationChannelId::from_string(&cmd.channel_id);

        self.channel_repo.delete(&channel_id).await?;

        info!("Notification channel deleted: {}", cmd.channel_id);

        Ok(())
    }
}

/// Test notification channel command handler
pub struct TestNotificationChannelHandler {
    channel_repo: Arc<dyn NotificationChannelRepository>,
}

impl TestNotificationChannelHandler {
    pub fn new(channel_repo: Arc<dyn NotificationChannelRepository>) -> Self {
        Self { channel_repo }
    }
}

#[async_trait]
impl CommandHandler<TestNotificationChannelCommand> for TestNotificationChannelHandler {
    type Result = TestNotificationChannelResult;

    async fn handle(
        &self,
        cmd: TestNotificationChannelCommand,
    ) -> Result<Self::Result, DomainError> {
        info!("Testing notification channel: {}", cmd.channel_id);

        let channel_id = NotificationChannelId::from_string(&cmd.channel_id);

        // Load channel
        let channel = self
            .channel_repo
            .find_by_id(&channel_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("Channel not found: {}", cmd.channel_id))
            })?;

        // Create sender
        let sender = create_sender(channel.config())?;

        // Test
        match sender.test().await {
            Ok(_) => {
                info!("Notification channel test successful: {}", cmd.channel_id);
                Ok(TestNotificationChannelResult {
                    success: true,
                    message: "测试通知发送成功".to_string(),
                })
            }
            Err(e) => {
                info!("Notification channel test failed: {}", e);
                Ok(TestNotificationChannelResult {
                    success: false,
                    message: format!("测试失败: {}", e),
                })
            }
        }
    }
}
