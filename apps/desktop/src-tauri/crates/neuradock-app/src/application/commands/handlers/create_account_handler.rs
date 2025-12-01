use async_trait::async_trait;
use chrono::{Duration, Utc};
use log::info;
use std::sync::Arc;

use crate::application::commands::account_commands::*;
use crate::application::commands::command_handler::CommandHandler;
use neuradock_domain::account::{Account, AccountRepository, Credentials};
use neuradock_domain::events::account_events::AccountCreated;
use neuradock_domain::events::EventBus;
use neuradock_domain::shared::{AccountId, DomainError, ProviderId};

/// Default session expiration duration (30 days)
const DEFAULT_SESSION_EXPIRATION_DAYS: i64 = 30;

/// Create account command handler
pub struct CreateAccountCommandHandler {
    account_repo: Arc<dyn AccountRepository>,
    event_bus: Arc<dyn EventBus>,
}

impl CreateAccountCommandHandler {
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
impl CommandHandler<CreateAccountCommand> for CreateAccountCommandHandler {
    type Result = CreateAccountResult;

    async fn handle(&self, cmd: CreateAccountCommand) -> Result<Self::Result, DomainError> {
        info!("Handling CreateAccountCommand for account: {}", cmd.name);

        // 1. Create credentials
        let credentials = Credentials::new(cmd.cookies.clone(), cmd.api_user);

        // 2. Create account aggregate
        let mut account = Account::new(
            cmd.name,
            ProviderId::from_string(&cmd.provider_id),
            credentials,
        )?;

        // 3. Set session expiration to 30 days from now
        let session_token = cmd.cookies
            .values()
            .next()
            .cloned()
            .unwrap_or_else(|| "session".to_string());
        let expires_at = Utc::now() + Duration::days(DEFAULT_SESSION_EXPIRATION_DAYS);
        account.update_session(session_token, expires_at);

        // 4. Set auto check-in configuration if provided
        if let Some(enabled) = cmd.auto_checkin_enabled {
            let hour = cmd.auto_checkin_hour.unwrap_or(9);
            let minute = cmd.auto_checkin_minute.unwrap_or(0);
            account.update_auto_checkin(enabled, hour, minute)?;
        }

        // 5. Save account
        self.account_repo.save(&account).await?;

        let account_id = account.id().clone();
        let name = account.name().to_string();
        let provider_id = account.provider_id().clone();
        let auto_checkin_enabled = account.auto_checkin_enabled();

        info!(
            "Account created successfully: {} ({}) - session expires in {} days",
            name,
            account_id.as_str(),
            DEFAULT_SESSION_EXPIRATION_DAYS
        );

        // 6. Publish domain event
        let event = AccountCreated {
            account_id: account_id.clone(),
            name,
            provider_id,
            auto_checkin_enabled,
            occurred_at: Utc::now(),
        };

        self.event_bus.publish(Box::new(event)).await?;

        // 7. Return result
        Ok(CreateAccountResult {
            account_id: account_id.as_str().to_string(),
        })
    }
}
