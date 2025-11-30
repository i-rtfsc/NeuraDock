use async_trait::async_trait;
use chrono::Utc;
use log::info;
use std::sync::Arc;

use crate::application::commands::account_commands::*;
use crate::application::commands::command_handler::CommandHandler;
use neuradock_domain::account::AccountRepository;
use neuradock_domain::events::account_events::AccountDeleted;
use neuradock_domain::events::EventBus;
use neuradock_domain::shared::{AccountId, DomainError};

/// Delete account command handler
pub struct DeleteAccountCommandHandler {
    account_repo: Arc<dyn AccountRepository>,
    event_bus: Arc<dyn EventBus>,
}

impl DeleteAccountCommandHandler {
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
impl CommandHandler<DeleteAccountCommand> for DeleteAccountCommandHandler {
    type Result = DeleteAccountResult;

    async fn handle(&self, cmd: DeleteAccountCommand) -> Result<Self::Result, DomainError> {
        info!("Handling DeleteAccountCommand for account: {}", cmd.account_id);

        let account_id = AccountId::from_string(&cmd.account_id);

        // 1. Verify account exists
        let account = self
            .account_repo
            .find_by_id(&account_id)
            .await?
            .ok_or_else(|| DomainError::AccountNotFound(cmd.account_id.clone()))?;

        let name = account.name().to_string();
        info!("Deleting account: {}", name);

        // 2. Delete account
        self.account_repo.delete(&account_id).await?;

        info!("Account deleted successfully: {}", name);

        // 3. Publish domain event
        let event = AccountDeleted {
            account_id,
            name,
            occurred_at: Utc::now(),
        };
        
        self.event_bus.publish(Box::new(event)).await?;

        Ok(DeleteAccountResult { success: true })
    }
}
