use std::sync::Arc;

use neuradock_domain::reminder_settings::ReminderSettingsRepository;
use neuradock_domain::shared::DomainError;

use crate::application::dtos::{ReminderSettingsDto, UpdateReminderSettingsInput};

pub struct ReminderSettingsService {
    repo: Arc<dyn ReminderSettingsRepository>,
}

impl ReminderSettingsService {
    pub fn new(repo: Arc<dyn ReminderSettingsRepository>) -> Self {
        Self { repo }
    }

    pub async fn get(&self) -> Result<ReminderSettingsDto, DomainError> {
        let settings = self.repo.get().await?;
        Ok(ReminderSettingsDto::from(&settings))
    }

    pub async fn update(
        &self,
        input: UpdateReminderSettingsInput,
    ) -> Result<ReminderSettingsDto, DomainError> {
        let settings = input.to_domain().map_err(DomainError::Validation)?;
        self.repo.save(&settings).await?;
        Ok(ReminderSettingsDto::from(&settings))
    }
}
