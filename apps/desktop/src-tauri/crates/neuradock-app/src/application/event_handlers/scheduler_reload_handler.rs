use async_trait::async_trait;
use log::{error, info};
use std::collections::HashMap;
use std::sync::Arc;

use crate::application::services::AutoCheckInScheduler;
use neuradock_domain::account::AccountRepository;
use neuradock_domain::check_in::Provider;
use neuradock_domain::events::account_events::*;
use neuradock_domain::events::event_bus::EventHandler;
use neuradock_domain::shared::DomainError;

/// Handler for account lifecycle events that trigger scheduler reload
/// This decouples the scheduler from the command handlers
#[derive(Clone)]
pub struct SchedulerReloadEventHandler {
    scheduler: Arc<AutoCheckInScheduler>,
    account_repo: Arc<dyn AccountRepository>,
    providers: HashMap<String, Provider>,
    app_handle: tauri::AppHandle,
}

impl SchedulerReloadEventHandler {
    pub fn new(
        scheduler: Arc<AutoCheckInScheduler>,
        account_repo: Arc<dyn AccountRepository>,
        providers: HashMap<String, Provider>,
        app_handle: tauri::AppHandle,
    ) -> Self {
        Self {
            scheduler,
            account_repo,
            providers,
            app_handle,
        }
    }

    async fn reload_schedules(&self) -> Result<(), DomainError> {
        info!("🔄 [SCHEDULER] Reloading schedules due to account change");
        
        self.scheduler
            .reload_schedules(
                self.providers.clone(),
                self.account_repo.clone(),
                self.app_handle.clone(),
            )
            .await
            .map_err(|e| {
                error!("❌ [SCHEDULER] Failed to reload schedules: {}", e);
                DomainError::Infrastructure(format!("Failed to reload schedules: {}", e))
            })?;
        
        info!("✅ [SCHEDULER] Schedules reloaded successfully");
        Ok(())
    }
}

#[async_trait]
impl EventHandler<AccountCreated> for SchedulerReloadEventHandler {
    async fn handle(&self, event: &AccountCreated) -> Result<(), DomainError> {
        info!(
            "🔔 [EVENT] AccountCreated: {} ({}) - auto_checkin_enabled: {}",
            event.name, event.account_id, event.auto_checkin_enabled
        );
        
        if event.auto_checkin_enabled {
            info!("🔄 Reloading scheduler for new account with auto check-in enabled");
            self.reload_schedules().await?;
        } else {
            info!("⏭️  Auto check-in not enabled for account, skipping scheduler reload");
        }
        
        Ok(())
    }
}

#[async_trait]
impl EventHandler<AccountUpdated> for SchedulerReloadEventHandler {
    async fn handle(&self, event: &AccountUpdated) -> Result<(), DomainError> {
        info!(
            "🔔 [EVENT] AccountUpdated: {} - auto_checkin_config_updated: {}",
            event.account_id, event.auto_checkin_config_updated
        );
        
        // Reload if auto check-in config was updated
        if event.auto_checkin_config_updated {
            info!("🔄 Reloading scheduler due to auto check-in config update");
            self.reload_schedules().await?;
        } else {
            info!("⏭️  Auto check-in config not updated, skipping scheduler reload");
        }
        
        Ok(())
    }
}

#[async_trait]
impl EventHandler<AccountDeleted> for SchedulerReloadEventHandler {
    async fn handle(&self, event: &AccountDeleted) -> Result<(), DomainError> {
        info!(
            "🔔 [EVENT] AccountDeleted: {} ({})",
            event.name, event.account_id
        );
        
        info!("🔄 Reloading scheduler to remove deleted account's schedule");
        self.reload_schedules().await?;
        Ok(())
    }
}

#[async_trait]
impl EventHandler<AccountToggled> for SchedulerReloadEventHandler {
    async fn handle(&self, event: &AccountToggled) -> Result<(), DomainError> {
        info!(
            "🔔 [EVENT] AccountToggled: {} - enabled: {}",
            event.account_id, event.enabled
        );
        
        info!("🔄 Reloading scheduler due to account toggle");
        self.reload_schedules().await?;
        Ok(())
    }
}
