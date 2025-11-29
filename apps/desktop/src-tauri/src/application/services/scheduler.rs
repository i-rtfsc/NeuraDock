use chrono::{Local, Timelike};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval_at, Duration, Instant};
use tracing::{error, info};

use crate::domain::account::AccountRepository;
use crate::domain::check_in::Provider;
use crate::domain::shared::AccountId;

pub struct AutoCheckInScheduler {
    account_repo: Arc<dyn AccountRepository>,
}

impl AutoCheckInScheduler {
    pub async fn new(
        account_repo: Arc<dyn AccountRepository>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self { account_repo })
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Auto check-in scheduler started (using tokio timer)");
        eprintln!("✓ Auto check-in scheduler started");
        Ok(())
    }

    pub async fn reload_schedules(
        &self,
        providers: HashMap<String, Provider>,
        account_repo: Arc<dyn AccountRepository>,
        app_handle: tauri::AppHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Reloading auto check-in schedules");
        eprintln!("🔄 Reloading auto check-in schedules...");

        let now = Local::now();
        eprintln!(
            "  📍 Current local time: {}",
            now.format("%Y-%m-%d %H:%M:%S %Z")
        );

        // Get all accounts with auto check-in enabled
        let accounts = account_repo.find_all().await?;
        eprintln!("  Found {} total accounts", accounts.len());

        let mut scheduled_count = 0;
        for account in accounts {
            eprintln!(
                "  Account: {} - enabled: {}, auto_checkin: {}",
                account.name(),
                account.is_enabled(),
                account.auto_checkin_enabled()
            );

            if account.auto_checkin_enabled() && account.is_enabled() {
                let provider_id = account.provider_id().as_str();
                if let Some(provider) = providers.get(provider_id) {
                    self.spawn_check_in_task(
                        account.id().clone(),
                        account.name().to_string(),
                        account.auto_checkin_hour(),
                        account.auto_checkin_minute(),
                        provider.clone(),
                        account_repo.clone(),
                        app_handle.clone(),
                    );
                    scheduled_count += 1;
                } else {
                    eprintln!(
                        "  ⚠ Provider '{}' not found for account {}",
                        provider_id,
                        account.name()
                    );
                }
            }
        }

        eprintln!("✓ Scheduled {} auto check-in jobs", scheduled_count);

        Ok(())
    }

    fn spawn_check_in_task(
        &self,
        account_id: AccountId,
        account_name: String,
        hour: u8,
        minute: u8,
        provider: Provider,
        account_repo: Arc<dyn AccountRepository>,
        app_handle: tauri::AppHandle,
    ) {
        eprintln!(
            "  ➕ Spawning task for '{}' at {}:{:02}",
            account_name, hour, minute
        );

        tokio::spawn(async move {
            loop {
                let now = Local::now();
                let target_hour = hour as u32;
                let target_minute = minute as u32;

                // Calculate next execution time
                let mut next_run = now
                    .date_naive()
                    .and_hms_opt(target_hour, target_minute, 0)
                    .unwrap()
                    .and_local_timezone(now.timezone())
                    .unwrap();

                // If the target time has already passed today, schedule for tomorrow
                if next_run <= now {
                    next_run = next_run + chrono::Duration::days(1);
                }

                let duration_until_next =
                    (next_run - now).to_std().unwrap_or(Duration::from_secs(60));

                eprintln!(
                    "     Next run for '{}': {} (in {} seconds)",
                    account_name,
                    next_run.format("%Y-%m-%d %H:%M:%S"),
                    duration_until_next.as_secs()
                );

                // Sleep until next execution
                tokio::time::sleep(duration_until_next).await;

                // Execute check-in
                eprintln!(
                    "⏰ [AUTO CHECK-IN] Executing for account: {} at {}",
                    account_name,
                    Local::now().format("%Y-%m-%d %H:%M:%S %Z")
                );
                info!("Executing auto check-in for account {}", account_name);

                use crate::application::services::CheckInExecutor;
                match CheckInExecutor::new(account_repo.clone(), true) {
                    Ok(executor) => {
                        match executor
                            .execute_check_in(account_id.as_str(), &provider)
                            .await
                        {
                            Ok(result) => {
                                if result.success {
                                    eprintln!(
                                        "✓ [AUTO CHECK-IN] Success for {}: {}",
                                        account_name, result.message
                                    );
                                    info!(
                                        "Auto check-in successful for account {}: {}",
                                        account_name, result.message
                                    );

                                    // Send notification
                                    use tauri_plugin_notification::NotificationExt;
                                    if let Err(e) = app_handle
                                        .notification()
                                        .builder()
                                        .title("Auto Check-in Success")
                                        .body(format!("{}: {}", account_name, result.message))
                                        .show()
                                    {
                                        error!("Failed to send notification: {}", e);
                                        eprintln!(
                                            "✗ [AUTO CHECK-IN] Failed to send notification: {}",
                                            e
                                        );
                                    }
                                } else {
                                    eprintln!(
                                        "✗ [AUTO CHECK-IN] Failed for {}: {}",
                                        account_name, result.message
                                    );
                                    error!(
                                        "Auto check-in failed for account {}: {}",
                                        account_name, result.message
                                    );
                                }
                            }
                            Err(e) => {
                                eprintln!("✗ [AUTO CHECK-IN] Error for {}: {}", account_name, e);
                                error!("Auto check-in failed for account {}: {}", account_name, e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("✗ [AUTO CHECK-IN] Failed to create executor: {}", e);
                        error!("Failed to create CheckInExecutor: {}", e);
                    }
                }
            }
        });
    }

    pub async fn shutdown(mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Shutting down auto check-in scheduler");
        Ok(())
    }
}
