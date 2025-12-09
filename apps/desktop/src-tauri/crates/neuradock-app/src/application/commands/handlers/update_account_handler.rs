use async_trait::async_trait;
use chrono::Utc;
use log::info;
use std::sync::Arc;

use crate::application::commands::account_commands::*;
use crate::application::commands::command_handler::CommandHandler;
use neuradock_domain::account::{Account, AccountRepository, Credentials};
use neuradock_domain::events::account_events::AccountUpdated;
use neuradock_domain::events::EventBus;
use neuradock_domain::shared::{AccountId, DomainError};

/// Update account command handler
pub struct UpdateAccountCommandHandler {
    account_repo: Arc<dyn AccountRepository>,
    event_bus: Arc<dyn EventBus>,
}

impl UpdateAccountCommandHandler {
    pub fn new(account_repo: Arc<dyn AccountRepository>, event_bus: Arc<dyn EventBus>) -> Self {
        Self {
            account_repo,
            event_bus,
        }
    }
}

#[async_trait]
impl CommandHandler<UpdateAccountCommand> for UpdateAccountCommandHandler {
    type Result = UpdateAccountResult;

    async fn handle(&self, cmd: UpdateAccountCommand) -> Result<Self::Result, DomainError> {
        info!(
            "Handling UpdateAccountCommand for account: {}",
            cmd.account_id
        );

        let account_id = AccountId::from_string(&cmd.account_id);

        // 1. Load account aggregate
        let mut account = self
            .account_repo
            .find_by_id(&account_id)
            .await?
            .ok_or_else(|| DomainError::AccountNotFound(cmd.account_id.clone()))?;

        let mut name_updated = None;
        let mut credentials_updated = false;
        let mut auto_checkin_config_updated = false;

        // 2. Update name if provided
        if let Some(name) = cmd.name {
            account.update_name(name.clone())?;
            name_updated = Some(name);
        }

        // 3. Update credentials if provided
        if let Some(cookies) = cmd.cookies {
            let api_user = cmd
                .api_user
                .unwrap_or_else(|| account.credentials().api_user().to_string());
            let credentials = Credentials::new(cookies.clone(), api_user);
            account.update_credentials(credentials)?;
            credentials_updated = true;

            // When cookies are updated, set session expiration to 30 days from now
            // This allows frontend to track when the session will expire
            account.refresh_session_with_default_expiration(&cookies);

            info!(
                "Session expiration set to {} days from now for account: {}",
                Account::DEFAULT_SESSION_EXPIRATION_DAYS,
                account.name()
            );
        }

        // 4. Update auto check-in configuration if provided
        if let Some(enabled) = cmd.auto_checkin_enabled {
            let hour = cmd.auto_checkin_hour.unwrap_or(account.auto_checkin_hour());
            let minute = cmd
                .auto_checkin_minute
                .unwrap_or(account.auto_checkin_minute());
            account.update_auto_checkin(enabled, hour, minute)?;
            auto_checkin_config_updated = true;
        }

        // 5. Save updated account
        self.account_repo.save(&account).await?;

        info!("Account updated successfully: {}", account.name());

        // 6. Publish domain event
        let event = AccountUpdated {
            account_id,
            name: name_updated,
            credentials_updated,
            auto_checkin_config_updated,
            occurred_at: Utc::now(),
        };

        self.event_bus.publish(Box::new(event)).await?;

        Ok(UpdateAccountResult { success: true })
    }
}
