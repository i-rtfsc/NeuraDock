//! The set of check-in runs currently in progress.
//!
//! Jobs are deliberately in memory: a run only exists while the app is running
//! it, and when the app stops there is nothing to resume. The `check_in_jobs`
//! table was dropped in 20250202000001_drop_unused_tables, so the old
//! `CheckInJob` aggregate was describing storage that no longer existed.
//!
//! Cancellation is cooperative and coarse on purpose: a batch checks the flag
//! between accounts. An in-flight HTTP request is not interrupted, and the job
//! reports how far it got.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use crate::application::dtos::RunningJobDto;
use neuradock_domain::shared::JobId;

struct RunningJob {
    account_ids: Vec<String>,
    started_at: DateTime<Utc>,
    processed: AtomicU32,
    cancelled: AtomicBool,
}

/// Registry of in-progress check-in runs.
#[derive(Default)]
pub struct CheckInJobRegistry {
    jobs: Mutex<HashMap<String, Arc<RunningJob>>>,
}

/// Handle for one run: its id, its cancellation flag, and how many accounts it
/// has finished.
pub struct CheckInJobHandle {
    id: String,
    job: Arc<RunningJob>,
}

impl CheckInJobHandle {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn is_cancelled(&self) -> bool {
        self.job.cancelled.load(Ordering::SeqCst)
    }

    /// Record that one account has been taken off the queue. A batch calls this
    /// as it starts each account, so the count grows while the run works.
    pub fn mark_processed(&self) {
        self.job.processed.fetch_add(1, Ordering::SeqCst);
    }
}

impl CheckInJobRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a run and return its handle.
    pub fn start(&self, account_ids: Vec<String>) -> CheckInJobHandle {
        let id = JobId::new().as_str().to_string();
        let job = Arc::new(RunningJob {
            account_ids,
            started_at: Utc::now(),
            processed: AtomicU32::new(0),
            cancelled: AtomicBool::new(false),
        });

        if let Ok(mut jobs) = self.jobs.lock() {
            jobs.insert(id.clone(), job.clone());
        }

        CheckInJobHandle { id, job }
    }

    /// Remove a finished run.
    pub fn finish(&self, handle: &CheckInJobHandle) {
        if let Ok(mut jobs) = self.jobs.lock() {
            jobs.remove(&handle.id);
        }
    }

    /// Ask a run to stop. Returns false when the id is unknown, which is what
    /// an already-finished job looks like.
    pub fn cancel(&self, job_id: &str) -> bool {
        let Ok(jobs) = self.jobs.lock() else {
            return false;
        };
        match jobs.get(job_id) {
            Some(job) => {
                job.cancelled.store(true, Ordering::SeqCst);
                true
            }
            None => false,
        }
    }

    pub fn list(&self) -> Vec<RunningJobDto> {
        let Ok(jobs) = self.jobs.lock() else {
            return Vec::new();
        };

        let mut listed: Vec<RunningJobDto> = jobs
            .iter()
            .map(|(id, job)| RunningJobDto {
                id: id.clone(),
                account_ids: job.account_ids.clone(),
                started_at: job.started_at.to_rfc3339(),
                processed: job.processed.load(Ordering::SeqCst),
                total: job.account_ids.len() as u32,
                cancelled: job.cancelled.load(Ordering::SeqCst),
            })
            .collect();

        listed.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        listed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_started_run_is_listed_and_can_be_cancelled() {
        let registry = CheckInJobRegistry::new();
        let handle = registry.start(vec!["a".to_string(), "b".to_string()]);

        let listed = registry.list();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, handle.id());
        assert_eq!(listed[0].total, 2);
        assert!(!listed[0].cancelled);

        assert!(registry.cancel(handle.id()));
        assert!(handle.is_cancelled());
        assert!(registry.list()[0].cancelled);

        handle.mark_processed();
        assert_eq!(registry.list()[0].processed, 1);
    }

    #[test]
    fn finishing_a_run_removes_it_and_cancelling_it_again_is_a_no_op() {
        let registry = CheckInJobRegistry::new();
        let handle = registry.start(vec!["a".to_string()]);
        registry.finish(&handle);

        assert!(registry.list().is_empty());
        assert!(!registry.cancel(handle.id()));
    }
}
