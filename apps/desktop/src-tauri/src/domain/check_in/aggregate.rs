use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::value_objects::{CheckInResult, CheckInStatus};
use crate::domain::shared::{AccountId, DomainError, JobId, ProviderId};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CheckInJob {
    id: JobId,
    account_id: AccountId,
    provider_id: ProviderId,
    status: CheckInStatus,
    scheduled_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    result: Option<CheckInResult>,
    error: Option<String>,
}

impl CheckInJob {
    pub fn new(
        account_id: AccountId,
        provider_id: ProviderId,
        scheduled_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: JobId::new(),
            account_id,
            provider_id,
            status: CheckInStatus::Pending,
            scheduled_at,
            started_at: None,
            completed_at: None,
            result: None,
            error: None,
        }
    }

    pub fn id(&self) -> &JobId {
        &self.id
    }

    pub fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    pub fn provider_id(&self) -> &ProviderId {
        &self.provider_id
    }

    pub fn status(&self) -> &CheckInStatus {
        &self.status
    }

    pub fn result(&self) -> Option<&CheckInResult> {
        self.result.as_ref()
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn start(&mut self) -> Result<(), DomainError> {
        if self.status != CheckInStatus::Pending {
            return Err(DomainError::Validation("Job is not pending".to_string()));
        }
        self.status = CheckInStatus::Running;
        self.started_at = Some(Utc::now());
        Ok(())
    }

    pub fn complete(&mut self, result: CheckInResult) -> Result<(), DomainError> {
        if self.status != CheckInStatus::Running {
            return Err(DomainError::Validation("Job is not running".to_string()));
        }
        self.status = CheckInStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.result = Some(result);
        Ok(())
    }

    pub fn fail(&mut self, error: String) -> Result<(), DomainError> {
        if self.status != CheckInStatus::Running && self.status != CheckInStatus::Pending {
            return Err(DomainError::Validation(
                "Job is not in valid state".to_string(),
            ));
        }
        self.status = CheckInStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error = Some(error);
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<(), DomainError> {
        if self.status == CheckInStatus::Completed || self.status == CheckInStatus::Failed {
            return Err(DomainError::Validation(
                "Cannot cancel completed job".to_string(),
            ));
        }
        self.status = CheckInStatus::Cancelled;
        self.completed_at = Some(Utc::now());
        Ok(())
    }
}
