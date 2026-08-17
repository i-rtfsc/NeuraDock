use super::types::{CheckInTaskConfig, TaskMetadata};
use chrono::Local;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::{error, info, warn};

impl super::AutoCheckInScheduler {
    pub(super) async fn spawn_check_in_task(&self, config: CheckInTaskConfig) {
        // Destructure config for easier use
        let CheckInTaskConfig {
            account_id,
            account_name,
            hour,
            minute,
            provider,
            account_repo,
            app_handle,
        } = config;

        info!(
            "➕ Spawning task for '{}' at {}:{:02}",
            account_name, hour, minute
        );

        // Clone account_id before moving it into the async closure
        let account_id_for_storage = account_id.clone();
        let account_name_clone = account_name.clone();

        // Clone task metadata for updating within the task
        let task_metadata = Arc::clone(&self.task_metadata);
        let deps = self.deps.clone();

        // Initialize metadata
        {
            let mut metadata = task_metadata.lock().await;
            metadata.insert(
                account_id_for_storage.clone(),
                TaskMetadata {
                    account_name: account_name_clone.clone(),
                    last_execution: None,
                },
            );
        }

        let handle = tokio::spawn(async move {
            loop {
                let now = Local::now();
                // Validate and clamp hour/minute to valid ranges to prevent panics
                let target_hour = (hour as u32).min(23);
                let target_minute = (minute as u32).min(59);

                // Warn if values were clamped
                if hour > 23 || minute > 59 {
                    error!(
                        "⚠️  Invalid schedule time for account '{}': {}:{} (clamped to {}:{})",
                        account_name, hour, minute, target_hour, target_minute
                    );
                }

                // Calculate next execution time with proper error handling
                let next_run = match now
                    .date_naive()
                    .and_hms_opt(target_hour, target_minute, 0)
                    .and_then(|dt| dt.and_local_timezone(now.timezone()).single())
                {
                    Some(mut next) => {
                        // If the target time has already passed today, schedule for tomorrow
                        if next <= now {
                            next += chrono::Duration::days(1);
                        }
                        next
                    }
                    None => {
                        error!(
                            "❌ Failed to calculate next run time for account '{}' with time {}:{}. Task will exit.",
                            account_name, target_hour, target_minute
                        );
                        break; // Exit the loop to stop this task
                    }
                };

                let duration_until_next =
                    (next_run - now).to_std().unwrap_or(Duration::from_secs(60));

                info!(
                    "Next run for '{}': {} (in {} seconds)",
                    account_name,
                    next_run.format("%Y-%m-%d %H:%M:%S"),
                    duration_until_next.as_secs()
                );

                // Sleep until next execution
                tokio::time::sleep(duration_until_next).await;

                // Execute check-in
                info!(
                    "⏰ [AUTO CHECK-IN] Executing for account: {} at {}",
                    account_name,
                    Local::now().format("%Y-%m-%d %H:%M:%S %Z")
                );

                // Update last execution time
                {
                    let mut metadata = task_metadata.lock().await;
                    if let Some(meta) = metadata.get_mut(&account_id) {
                        meta.last_execution = Some(chrono::Utc::now());
                    }
                }

                use crate::application::services::CheckInExecutor;
                // Resolve the proxy per run rather than at spawn time, so a
                // settings change takes effect without restarting the app.
                // `proxy_url()` already yields None when the user's toggle is
                // off, so honouring the setting is just a matter of asking.
                // This path previously called `CheckInExecutor::new`, which
                // hardcoded no proxy and so ignored the toggle entirely.
                //
                // A read failure here means the setting is unknown, not off.
                // Skip the run, matching the manual paths, which propagate the
                // same error rather than falling back to a direct connection.
                let proxy_url = match deps.proxy_config.get().await {
                    Ok(config) => config.proxy_url(),
                    Err(e) => {
                        warn!(
                            "[AUTO CHECK-IN] Could not read proxy settings for '{}': {}. \
                             Skipping this run; will retry on the next tick.",
                            account_name, e
                        );
                        continue;
                    }
                };

                match CheckInExecutor::with_proxy(account_repo.clone(), true, proxy_url) {
                    Ok(executor) => {
                        let executor = executor
                            .with_balance_history(deps.balance_history.clone())
                            .with_waf_cookies_repo(deps.waf_cookies.clone());
                        match executor
                            .execute_check_in(account_id.as_str(), &provider)
                            .await
                        {
                            Ok(result) => {
                                if result.success {
                                    info!(
                                        "✅ [AUTO CHECK-IN] Success for {}: {}",
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
                                        error!(
                                            "❌ [AUTO CHECK-IN] Failed to send notification: {}",
                                            e
                                        );
                                    }
                                } else {
                                    error!(
                                        "❌ [AUTO CHECK-IN] Failed for {}: {}",
                                        account_name, result.message
                                    );
                                }
                            }
                            Err(e) => {
                                error!("❌ [AUTO CHECK-IN] Error for {}: {}", account_name, e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("❌ [AUTO CHECK-IN] Failed to create executor: {}", e);
                    }
                }
            }
        });

        // Store the task handle for later cleanup
        let mut tasks = self.tasks.lock().await;

        // If there was an old task for this account, abort it
        if let Some(old_handle) = tasks.insert(account_id_for_storage.clone(), handle) {
            warn!(
                "⚠️  Aborting old task for account: {}",
                account_id_for_storage.as_str()
            );
            old_handle.abort();
        }

        info!(
            "✅ Task registered for account: {}",
            account_id_for_storage.as_str()
        );
    }
}
