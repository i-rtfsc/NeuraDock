use async_trait::async_trait;

use super::ReminderSettings;
use crate::shared::DomainError;

#[async_trait]
pub trait ReminderSettingsRepository: Send + Sync {
    /// Read the singleton. Returns the default when the row doesn't exist.
    async fn get(&self) -> Result<ReminderSettings, DomainError>;

    /// Persist (upsert).
    async fn save(&self, settings: &ReminderSettings) -> Result<(), DomainError>;
}
