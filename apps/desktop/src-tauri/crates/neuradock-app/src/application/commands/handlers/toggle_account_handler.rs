use async_trait::async_trait;
use chrono::Utc;
use log::info;
use std::sync::Arc;

use crate::application::commands::account_commands::*;
use crate::application::commands::command_handler::CommandHandler;
use neuradock_domain::account::AccountRepository;
use neuradock_domain::events::account_events::AccountToggled;
use neuradock_domain::events::EventBus;
use neuradock_domain::shared::{AccountId, DomainError};

/// Toggle account command handler
pub struct ToggleAccountCommandHandler {
    account_repo: Arc<dyn AccountRepository>,
    event_bus: Arc<dyn EventBus>,
}

impl ToggleAccountCommandHandler {
    pub fn new(
        account_repo: Arc<dyn AccountRepository>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        Self {
            account_repo,
            event_bus,
        }
    }
}

#[async_trait]
impl CommandHandler<ToggleAccountCommand> for ToggleAccountCommandHandler {
    type Result = ToggleAccountResult;

    async fn handle(&self, cmd: ToggleAccountCommand) -> Result<Self::Result, DomainError> {
        info!(
            "Handling ToggleAccountCommand for account: {} (enabled: {})",
            cmd.account_id, cmd.enabled
        );

        let account_id = AccountId::from_string(&cmd.account_id);

        // 1. Load account aggregate
        let mut account = self
            .account_repo
            .find_by_id(&account_id)
            .await?
            .ok_or_else(|| DomainError::AccountNotFound(cmd.account_id.clone()))?;

        // 2. Toggle account
        account.toggle(cmd.enabled);

        // 3. Save updated account
        self.account_repo.save(&account).await?;

        info!(
            "Account {} {}",
            account.name(),
            if cmd.enabled { "enabled" } else { "disabled" }
        );

        // 4. Publish domain event
        let event = AccountToggled {
            account_id,
            enabled: cmd.enabled,
            occurred_at: Utc::now(),
        };
        
        self.event_bus.publish(Box::new(event)).await?;

        Ok(ToggleAccountResult { success: true })
    }
}
