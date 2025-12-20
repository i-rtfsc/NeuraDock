use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;

use neuradock_domain::notification::{
    ChannelConfig, ChannelType, NotificationChannel, NotificationChannelId,
    NotificationChannelRepository,
};
use neuradock_domain::shared::DomainError;

use crate::persistence::RepositoryErrorMapper;

#[derive(FromRow)]
struct NotificationChannelRow {
    id: String,
    channel_type: String,
    config: String,
    enabled: bool,
    created_at: DateTime<Utc>,
}

impl NotificationChannelRow {
    #[allow(clippy::wrong_self_convention)]
    fn to_domain(self) -> Result<NotificationChannel, DomainError> {
        let id = NotificationChannelId::from_string(&self.id);
        let channel_type = ChannelType::from_str(&self.channel_type)?;
        let config = ChannelConfig::from_json(&self.config)?;

        Ok(NotificationChannel::from_persistence(
            id,
            channel_type,
            config,
            self.enabled,
            self.created_at,
        ))
    }
}

pub struct SqliteNotificationChannelRepository {
    pool: Arc<SqlitePool>,
}

impl SqliteNotificationChannelRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NotificationChannelRepository for SqliteNotificationChannelRepository {
    async fn save(&self, channel: &NotificationChannel) -> Result<(), DomainError> {
        let config_json = channel.config().to_json()?;

        sqlx::query(
            r#"
            INSERT INTO notification_channels (id, channel_type, config, enabled, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
        )
        .bind(channel.id().as_str())
        .bind(channel.channel_type().as_str())
        .bind(&config_json)
        .bind(channel.is_enabled())
        .bind(channel.created_at())
        .execute(&*self.pool)
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "notification_channels"))?;

        Ok(())
    }

    async fn find_by_id(
        &self,
        id: &NotificationChannelId,
    ) -> Result<Option<NotificationChannel>, DomainError> {
        let row: Option<NotificationChannelRow> = sqlx::query_as(
            r#"
            SELECT id, channel_type, config, enabled, created_at
            FROM notification_channels
            WHERE id = ?1
            "#,
        )
        .bind(id.as_str())
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "notification_channels"))?;

        match row {
            Some(r) => Ok(Some(r.to_domain()?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> Result<Vec<NotificationChannel>, DomainError> {
        let rows: Vec<NotificationChannelRow> = sqlx::query_as(
            r#"
            SELECT id, channel_type, config, enabled, created_at
            FROM notification_channels
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "notification_channels"))?;

        rows.into_iter().map(|r| r.to_domain()).collect()
    }

    async fn find_all_enabled(&self) -> Result<Vec<NotificationChannel>, DomainError> {
        let rows: Vec<NotificationChannelRow> = sqlx::query_as(
            r#"
            SELECT id, channel_type, config, enabled, created_at
            FROM notification_channels
            WHERE enabled = 1
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "notification_channels"))?;

        rows.into_iter().map(|r| r.to_domain()).collect()
    }

    async fn update(&self, channel: &NotificationChannel) -> Result<(), DomainError> {
        let config_json = channel.config().to_json()?;

        let result = sqlx::query(
            r#"
            UPDATE notification_channels
            SET channel_type = ?1, config = ?2, enabled = ?3
            WHERE id = ?4
            "#,
        )
        .bind(channel.channel_type().as_str())
        .bind(&config_json)
        .bind(channel.is_enabled())
        .bind(channel.id().as_str())
        .execute(&*self.pool)
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "notification_channels"))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!(
                "Notification channel not found: {}",
                channel.id()
            )));
        }

        Ok(())
    }

    async fn delete(&self, id: &NotificationChannelId) -> Result<(), DomainError> {
        let result = sqlx::query(
            r#"
            DELETE FROM notification_channels
            WHERE id = ?1
            "#,
        )
        .bind(id.as_str())
        .execute(&*self.pool)
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "notification_channels"))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!(
                "Notification channel not found: {}",
                id
            )));
        }

        Ok(())
    }
}
