mod health_check;
mod task_manager;
mod task_spawner;
mod types;

use neuradock_domain::proxy_config::ProxyConfigRepository;
use neuradock_domain::shared::AccountId;
use neuradock_domain::waf_cookies::WafCookiesRepository;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::info;

use crate::application::services::BalanceHistoryService;
use types::TaskMetadata;

/// Dependencies the scheduler hands to each check-in it runs.
///
/// The unattended path used to build its executor with nothing but the account
/// repository, which silently opted it out of three things every interactive
/// caller got: history recording, the user's proxy, and the WAF cookie cache.
#[derive(Clone)]
pub struct SchedulerDeps {
    pub balance_history: Arc<BalanceHistoryService>,
    pub proxy_config: Arc<dyn ProxyConfigRepository>,
    pub waf_cookies: Arc<dyn WafCookiesRepository>,
}

pub struct AutoCheckInScheduler {
    /// Active tasks mapped by account ID
    /// Using Mutex to allow modification from multiple contexts
    tasks: Arc<Mutex<HashMap<AccountId, JoinHandle<()>>>>,
    /// Task metadata for health monitoring
    task_metadata: Arc<Mutex<HashMap<AccountId, TaskMetadata>>>,
    /// Health check task handle
    health_check_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// Shared dependencies for scheduled check-ins
    deps: SchedulerDeps,
}

impl AutoCheckInScheduler {
    pub async fn new(deps: SchedulerDeps) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            task_metadata: Arc::new(Mutex::new(HashMap::new())),
            health_check_handle: Arc::new(Mutex::new(None)),
            deps,
        })
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("✅ Auto check-in scheduler started (using tokio timer)");

        // Start health check task
        self.start_health_check_task().await;

        Ok(())
    }
}
