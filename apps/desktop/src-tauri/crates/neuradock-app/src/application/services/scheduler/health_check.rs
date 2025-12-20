use std::sync::Arc;
use tokio::time::Duration;
use tracing::{error, info, warn};

impl super::AutoCheckInScheduler {
    /// Start health check background task to monitor scheduled tasks
    pub(super) async fn start_health_check_task(&self) {
        let tasks = Arc::clone(&self.tasks);
        let metadata = Arc::clone(&self.task_metadata);

        let handle = tokio::spawn(async move {
            let mut check_interval = tokio::time::interval(Duration::from_secs(300)); // Check every 5 minutes

            loop {
                check_interval.tick().await;

                let tasks_lock = tasks.lock().await;
                let mut metadata_lock = metadata.lock().await;

                let mut dead_tasks = Vec::new();

                for (account_id, handle) in tasks_lock.iter() {
                    if handle.is_finished() {
                        warn!(
                            "🔴 Health Check: Task for account {} has terminated unexpectedly",
                            account_id.as_str()
                        );
                        dead_tasks.push(account_id.clone());
                    } else if let Some(meta) = metadata_lock.get(account_id) {
                        // Check if task hasn't executed for more than 25 hours (should execute daily)
                        if let Some(last_exec) = meta.last_execution {
                            let elapsed = chrono::Utc::now() - last_exec;
                            if elapsed > chrono::Duration::hours(25) {
                                warn!(
                                    "⚠️  Health Check: Task for '{}' hasn't executed in {} hours",
                                    meta.account_name,
                                    elapsed.num_hours()
                                );
                            }
                        }
                    }
                }

                // Remove metadata for dead tasks
                for account_id in dead_tasks {
                    metadata_lock.remove(&account_id);
                    error!(
                        "🔴 Health Check: Removed dead task metadata for account {}",
                        account_id.as_str()
                    );
                }

                drop(tasks_lock);
                drop(metadata_lock);
            }
        });

        let mut health_check = self.health_check_handle.lock().await;
        *health_check = Some(handle);

        info!("✅ Health check task started (checking every 5 minutes)");
    }
}
