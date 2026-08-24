use async_trait::async_trait;
use std::sync::Arc;

use neuradock_domain::reminder_settings::{ReminderSettings, ReminderSettingsRepository};
use neuradock_domain::shared::DomainError;

use crate::persistence::repository_base::SqliteRepositoryBase;

pub struct SqliteReminderSettingsRepository {
    base: SqliteRepositoryBase,
}

impl SqliteReminderSettingsRepository {
    pub fn new(pool: Arc<sqlx::SqlitePool>) -> Self {
        Self {
            base: SqliteRepositoryBase::new(pool),
        }
    }
}

#[async_trait]
impl ReminderSettingsRepository for SqliteReminderSettingsRepository {
    async fn get(&self) -> Result<ReminderSettings, DomainError> {
        let query = "SELECT enabled, times, channels, stop_when_all_done FROM reminder_settings WHERE id = 1";

        let row: Result<Option<(bool, String, String, bool)>, _> = self
            .base
            .fetch_optional(sqlx::query_as(query), "Get reminder settings")
            .await;

        Ok(match row? {
            Some((enabled, times, channels, stop_when_all_done)) => {
                ReminderSettings::from_storage(enabled, &times, &channels, stop_when_all_done)
            }
            None => ReminderSettings::default(),
        })
    }

    async fn save(&self, settings: &ReminderSettings) -> Result<(), DomainError> {
        let query = r#"
            INSERT INTO reminder_settings
                (id, enabled, times, channels, stop_when_all_done, updated_at)
            VALUES (1, ?1, ?2, ?3, ?4, datetime('now'))
            ON CONFLICT(id) DO UPDATE SET
                enabled = excluded.enabled,
                times = excluded.times,
                channels = excluded.channels,
                stop_when_all_done = excluded.stop_when_all_done,
                updated_at = excluded.updated_at
        "#;

        self.base
            .execute(
                sqlx::query(query)
                    .bind(settings.enabled())
                    .bind(settings.times_to_storage())
                    .bind(settings.channels_to_storage())
                    .bind(settings.stop_when_all_done()),
                "Save reminder settings",
            )
            .await?;

        Ok(())
    }
}
