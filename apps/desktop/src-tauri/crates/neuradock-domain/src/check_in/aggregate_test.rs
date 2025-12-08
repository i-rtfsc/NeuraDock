#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::shared::{AccountId, ProviderId};
    use chrono::Utc;

    #[test]
    fn test_create_check_in_job() {
        let account_id = AccountId::new();
        let provider_id = ProviderId::from_string("anyrouter");
        let scheduled_at = Utc::now();

        let job = CheckInJob::new(account_id.clone(), provider_id.clone(), scheduled_at);

        assert_eq!(job.account_id(), &account_id);
        assert_eq!(job.provider_id(), &provider_id);
        assert_eq!(job.status(), &CheckInStatus::Pending);
        assert!(job.result().is_none());
        assert!(job.error().is_none());
    }

    #[test]
    fn test_start_job_from_pending() {
        let account_id = AccountId::new();
        let provider_id = ProviderId::from_string("anyrouter");
        let scheduled_at = Utc::now();

        let mut job = CheckInJob::new(account_id, provider_id, scheduled_at);

        let result = job.start();

        assert!(result.is_ok());
        assert_eq!(job.status(), &CheckInStatus::Running);
    }

    #[test]
    fn test_start_job_from_non_pending_fails() {
        let account_id = AccountId::new();
        let provider_id = ProviderId::from_string("anyrouter");
        let scheduled_at = Utc::now();

        let mut job = CheckInJob::new(account_id, provider_id, scheduled_at);
        job.start().unwrap();

        // Try to start again
        let result = job.start();

        assert!(result.is_err());
        assert_eq!(job.status(), &CheckInStatus::Running); // Status should not change
    }

    #[test]
    fn test_complete_job_from_running() {
        let account_id = AccountId::new();
        let provider_id = ProviderId::from_string("anyrouter");
        let scheduled_at = Utc::now();

        let mut job = CheckInJob::new(account_id, provider_id, scheduled_at);
        job.start().unwrap();

        let balance = Balance::new(100.0, 50.0);
        let check_in_result = CheckInResult {
            success: true,
            balance: Some(balance),
            message: Some("Success".to_string()),
        };

        let result = job.complete(check_in_result);

        assert!(result.is_ok());
        assert_eq!(job.status(), &CheckInStatus::Completed);
        assert!(job.result().is_some());
        assert_eq!(job.result().unwrap().success, true);
    }

    #[test]
    fn test_complete_job_from_non_running_fails() {
        let account_id = AccountId::new();
        let provider_id = ProviderId::from_string("anyrouter");
        let scheduled_at = Utc::now();

        let mut job = CheckInJob::new(account_id, provider_id, scheduled_at);

        let balance = Balance::new(100.0, 50.0);
        let check_in_result = CheckInResult {
            success: true,
            balance: Some(balance),
            message: Some("Success".to_string()),
        };

        // Try to complete without starting
        let result = job.complete(check_in_result);

        assert!(result.is_err());
        assert_eq!(job.status(), &CheckInStatus::Pending); // Status should not change
    }

    #[test]
    fn test_fail_job_from_running() {
        let account_id = AccountId::new();
        let provider_id = ProviderId::from_string("anyrouter");
        let scheduled_at = Utc::now();

        let mut job = CheckInJob::new(account_id, provider_id, scheduled_at);
        job.start().unwrap();

        let result = job.fail("Network error".to_string());

        assert!(result.is_ok());
        assert_eq!(job.status(), &CheckInStatus::Failed);
        assert_eq!(job.error(), Some("Network error"));
    }

    #[test]
    fn test_fail_job_from_pending() {
        let account_id = AccountId::new();
        let provider_id = ProviderId::from_string("anyrouter");
        let scheduled_at = Utc::now();

        let mut job = CheckInJob::new(account_id, provider_id, scheduled_at);

        let result = job.fail("Pre-check failed".to_string());

        assert!(result.is_ok());
        assert_eq!(job.status(), &CheckInStatus::Failed);
        assert_eq!(job.error(), Some("Pre-check failed"));
    }

    #[test]
    fn test_fail_job_from_completed_fails() {
        let account_id = AccountId::new();
        let provider_id = ProviderId::from_string("anyrouter");
        let scheduled_at = Utc::now();

        let mut job = CheckInJob::new(account_id, provider_id, scheduled_at);
        job.start().unwrap();

        let balance = Balance::new(100.0, 50.0);
        let check_in_result = CheckInResult {
            success: true,
            balance: Some(balance),
            message: Some("Success".to_string()),
        };
        job.complete(check_in_result).unwrap();

        // Try to fail after completion
        let result = job.fail("Error".to_string());

        assert!(result.is_err());
        assert_eq!(job.status(), &CheckInStatus::Completed); // Status should not change
    }

    #[test]
    fn test_cancel_job_from_pending() {
        let account_id = AccountId::new();
        let provider_id = ProviderId::from_string("anyrouter");
        let scheduled_at = Utc::now();

        let mut job = CheckInJob::new(account_id, provider_id, scheduled_at);

        let result = job.cancel();

        assert!(result.is_ok());
        assert_eq!(job.status(), &CheckInStatus::Cancelled);
    }

    #[test]
    fn test_cancel_job_from_running() {
        let account_id = AccountId::new();
        let provider_id = ProviderId::from_string("anyrouter");
        let scheduled_at = Utc::now();

        let mut job = CheckInJob::new(account_id, provider_id, scheduled_at);
        job.start().unwrap();

        let result = job.cancel();

        assert!(result.is_ok());
        assert_eq!(job.status(), &CheckInStatus::Cancelled);
    }

    #[test]
    fn test_cancel_job_from_completed_fails() {
        let account_id = AccountId::new();
        let provider_id = ProviderId::from_string("anyrouter");
        let scheduled_at = Utc::now();

        let mut job = CheckInJob::new(account_id, provider_id, scheduled_at);
        job.start().unwrap();

        let balance = Balance::new(100.0, 50.0);
        let check_in_result = CheckInResult {
            success: true,
            balance: Some(balance),
            message: Some("Success".to_string()),
        };
        job.complete(check_in_result).unwrap();

        // Try to cancel after completion
        let result = job.cancel();

        assert!(result.is_err());
        assert_eq!(job.status(), &CheckInStatus::Completed); // Status should not change
    }

    #[test]
    fn test_cancel_job_from_failed_fails() {
        let account_id = AccountId::new();
        let provider_id = ProviderId::from_string("anyrouter");
        let scheduled_at = Utc::now();

        let mut job = CheckInJob::new(account_id, provider_id, scheduled_at);
        job.start().unwrap();
        job.fail("Error".to_string()).unwrap();

        // Try to cancel after failure
        let result = job.cancel();

        assert!(result.is_err());
        assert_eq!(job.status(), &CheckInStatus::Failed); // Status should not change
    }

    #[test]
    fn test_complete_job_with_failure_result() {
        let account_id = AccountId::new();
        let provider_id = ProviderId::from_string("anyrouter");
        let scheduled_at = Utc::now();

        let mut job = CheckInJob::new(account_id, provider_id, scheduled_at);
        job.start().unwrap();

        let check_in_result = CheckInResult {
            success: false,
            balance: None,
            message: Some("Check-in failed".to_string()),
        };

        let result = job.complete(check_in_result);

        assert!(result.is_ok());
        assert_eq!(job.status(), &CheckInStatus::Completed);
        assert!(job.result().is_some());
        assert_eq!(job.result().unwrap().success, false);
    }
}
