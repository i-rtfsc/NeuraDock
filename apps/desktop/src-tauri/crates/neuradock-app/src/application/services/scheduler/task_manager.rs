use super::types::{BalanceObservationTaskConfig, CheckInTaskConfig};
use chrono::Local;
use neuradock_domain::account::AccountRepository;
use neuradock_domain::check_in::Provider;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

impl super::AutoCheckInScheduler {
    /// Stop all scheduled tasks
    pub async fn stop_all_tasks(&self) {
        // Stop health check task first
        let mut health_check = self.health_check_handle.lock().await;
        if let Some(handle) = health_check.take() {
            handle.abort();
            info!("🛑 Health check task stopped");
        }
        drop(health_check);

        let mut tasks = self.tasks.lock().await;
        let mut metadata = self.task_metadata.lock().await;

        info!("🛑 Stopping {} scheduled tasks...", tasks.len());

        for (account_id, handle) in tasks.drain() {
            info!("  ⏹️  Stopping task for account: {}", account_id.as_str());
            handle.abort();
        }

        metadata.clear();

        info!("✅ All scheduled tasks stopped");
    }

    /// Get count of active tasks
    pub async fn active_task_count(&self) -> usize {
        self.tasks.lock().await.len()
    }

    #[instrument(skip(self, providers, account_repo, app_handle))]
    pub async fn reload_schedules(
        &self,
        providers: HashMap<String, Provider>,
        account_repo: Arc<dyn AccountRepository>,
        app_handle: tauri::AppHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("🔄 Reloading auto check-in schedules");

        // 1. Stop all existing tasks first
        let existing_count = self.active_task_count().await;
        if existing_count > 0 {
            info!(
                "🔄 Stopping {} existing scheduled tasks before reload...",
                existing_count
            );
            self.stop_all_tasks().await;
        }

        let now = Local::now();
        info!(
            "📍 Current local time: {}",
            now.format("%Y-%m-%d %H:%M:%S %Z")
        );

        // 2. Get only enabled accounts, then filter by auto check-in.
        let accounts = account_repo.find_enabled().await?;
        info!("Found {} enabled accounts", accounts.len());

        let mut scheduled_count = 0;
        let mut observing_count = 0;
        let mut skipped_count = 0;
        for account in accounts {
            debug!(
                "Account: {} - enabled: {}, auto_checkin: {}",
                account.name(),
                account.is_enabled(),
                account.auto_checkin_enabled()
            );

            if account.auto_checkin_enabled() {
                let provider_id = account.provider_id().as_str();
                if let Some(provider) = providers.get(provider_id) {
                    // A provider the app cannot post to is still worth watching:
                    // an observation-only provider is one the user checks in for
                    // by hand, and the balance read is how that reaches the ledger.
                    match super::types::scheduled_task_kind(provider) {
                        super::types::ScheduledTaskKind::CheckIn => {
                            self.spawn_check_in_task(CheckInTaskConfig {
                                account_id: account.id().clone(),
                                account_name: account.name().to_string(),
                                hour: account.auto_checkin_hour(),
                                minute: account.auto_checkin_minute(),
                                provider: provider.clone(),
                                account_repo: account_repo.clone(),
                                app_handle: app_handle.clone(),
                            })
                            .await;
                            scheduled_count += 1;
                        }
                        super::types::ScheduledTaskKind::ObserveBalance => {
                            self.spawn_balance_observation_task(BalanceObservationTaskConfig {
                                account_id: account.id().clone(),
                                account_name: account.name().to_string(),
                                balance_service: self.deps.balance.clone(),
                            })
                            .await;
                            observing_count += 1;
                        }
                        super::types::ScheduledTaskKind::None => {
                            skipped_count += 1;
                            info!(
                                "Skipping '{}': provider '{}' is {:?} and cannot be automated",
                                account.name(),
                                provider_id,
                                provider.check_in_mode()
                            );
                        }
                    }
                } else {
                    warn!(
                        "Provider '{}' not found for account {}",
                        provider_id,
                        account.name()
                    );
                }
            }
        }

        info!(
            "✅ Scheduled {scheduled_count} auto check-in and {observing_count} balance observation job(s), skipped {skipped_count}"
        );

        Ok(())
    }
}
