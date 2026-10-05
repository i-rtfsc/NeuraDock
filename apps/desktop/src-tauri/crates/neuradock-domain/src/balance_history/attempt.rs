use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::{reward_to_cents, CheckInSource};
use crate::shared::{AccountId, DomainError, ProviderId};

/// Stable vocabulary for `check_in_attempts.error_code`.
///
/// Error codes are what the UI and the statistics endpoints group by, so they
/// must never carry an upstream response body or a formatted error string.
pub mod error_codes {
    pub const WAF_CHALLENGE: &str = "waf_challenge";
    pub const NETWORK: &str = "network";
    pub const TIMEOUT: &str = "timeout";
    pub const HTTP_STATUS: &str = "http_status";
    pub const INVALID_RESPONSE: &str = "invalid_response";
    pub const BALANCE_UNCONFIRMED: &str = "balance_unconfirmed";
    pub const GUARD_BLOCKED: &str = "guard_blocked";
    pub const CANCELLED: &str = "cancelled";
    pub const INTERNAL: &str = "internal";
}

/// What started an attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CheckInTrigger {
    Manual,
    Batch,
    Scheduler,
    Observe,
}

impl CheckInTrigger {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Batch => "batch",
            Self::Scheduler => "scheduler",
            Self::Observe => "observe",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "manual" => Ok(Self::Manual),
            "batch" => Ok(Self::Batch),
            "scheduler" => Ok(Self::Scheduler),
            "observe" => Ok(Self::Observe),
            other => Err(DomainError::Validation(format!(
                "Invalid check-in trigger: {other}"
            ))),
        }
    }
}

/// How an attempt ended. `None` on the entity means it is still open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CheckInOutcome {
    /// The provider accepted the check-in.
    Success,
    /// Evidence already existed for the quota day, so nothing was sent.
    AlreadyConfirmed,
    /// Worth trying again later today (network, 5xx, WAF).
    FailedRetryable,
    /// Trying again today will not help (bad credentials, 4xx).
    FailedPermanent,
    WafBlocked,
    /// The guard refused the attempt (in flight, backoff, interval).
    SkippedGuard,
    Cancelled,
}

impl CheckInOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::AlreadyConfirmed => "already_confirmed",
            Self::FailedRetryable => "failed_retryable",
            Self::FailedPermanent => "failed_permanent",
            Self::WafBlocked => "waf_blocked",
            Self::SkippedGuard => "skipped_guard",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "success" => Ok(Self::Success),
            "already_confirmed" => Ok(Self::AlreadyConfirmed),
            "failed_retryable" => Ok(Self::FailedRetryable),
            "failed_permanent" => Ok(Self::FailedPermanent),
            "waf_blocked" => Ok(Self::WafBlocked),
            "skipped_guard" => Ok(Self::SkippedGuard),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(DomainError::Validation(format!(
                "Invalid check-in outcome: {other}"
            ))),
        }
    }

    /// Whether this outcome counts against the success rate.
    pub fn is_failure(&self) -> bool {
        matches!(
            self,
            Self::FailedRetryable | Self::FailedPermanent | Self::WafBlocked
        )
    }

    /// Whether the next attempt for the same day should wait (exponential
    /// backoff). A permanent failure is not retried today at all, so it does
    /// not participate in backoff -- it is simply left alone.
    pub fn backs_off(&self) -> bool {
        matches!(self, Self::FailedRetryable | Self::WafBlocked)
    }
}

/// Everything needed to close an open attempt.
#[derive(Debug, Clone)]
pub struct AttemptCompletion {
    pub outcome: CheckInOutcome,
    pub finished_at: DateTime<Utc>,
    pub error_code: Option<String>,
    pub http_status: Option<i64>,
    pub reward: Option<f64>,
    pub source: Option<CheckInSource>,
}

/// One operation against a provider. Attempts are operational telemetry: they
/// are recorded whether or not they produce evidence, which is what lets the
/// app back off, single-flight and report history.
#[derive(Debug, Clone)]
pub struct CheckInAttempt {
    id: Option<i64>,
    account_id: AccountId,
    provider_id: ProviderId,
    quota_day: NaiveDate,
    trigger: CheckInTrigger,
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    outcome: Option<CheckInOutcome>,
    error_code: Option<String>,
    http_status: Option<i64>,
    duration_ms: Option<i64>,
    reward_cents: Option<i64>,
    source: Option<CheckInSource>,
}

impl CheckInAttempt {
    pub fn start(
        account_id: AccountId,
        provider_id: ProviderId,
        quota_day: NaiveDate,
        trigger: CheckInTrigger,
        started_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: None,
            account_id,
            provider_id,
            quota_day,
            trigger,
            started_at,
            finished_at: None,
            outcome: None,
            error_code: None,
            http_status: None,
            duration_ms: None,
            reward_cents: None,
            source: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: i64,
        account_id: AccountId,
        provider_id: ProviderId,
        quota_day: NaiveDate,
        trigger: CheckInTrigger,
        started_at: DateTime<Utc>,
        finished_at: Option<DateTime<Utc>>,
        outcome: Option<CheckInOutcome>,
        error_code: Option<String>,
        http_status: Option<i64>,
        duration_ms: Option<i64>,
        reward_cents: Option<i64>,
        source: Option<CheckInSource>,
    ) -> Self {
        Self {
            id: Some(id),
            account_id,
            provider_id,
            quota_day,
            trigger,
            started_at,
            finished_at,
            outcome,
            error_code,
            http_status,
            duration_ms,
            reward_cents,
            source,
        }
    }

    pub fn finish(&mut self, completion: AttemptCompletion) {
        self.finished_at = Some(completion.finished_at);
        self.outcome = Some(completion.outcome);
        self.error_code = completion.error_code;
        self.http_status = completion.http_status;
        self.reward_cents = completion.reward.map(reward_to_cents);
        self.source = completion.source;
        self.duration_ms = Some(
            (completion.finished_at - self.started_at)
                .num_milliseconds()
                .max(0),
        );
    }

    pub fn id(&self) -> Option<i64> {
        self.id
    }

    pub fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    pub fn provider_id(&self) -> &ProviderId {
        &self.provider_id
    }

    pub fn quota_day(&self) -> NaiveDate {
        self.quota_day
    }

    pub fn trigger(&self) -> CheckInTrigger {
        self.trigger
    }

    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    pub fn finished_at(&self) -> Option<DateTime<Utc>> {
        self.finished_at
    }

    pub fn outcome(&self) -> Option<CheckInOutcome> {
        self.outcome
    }

    pub fn error_code(&self) -> Option<&str> {
        self.error_code.as_deref()
    }

    pub fn http_status(&self) -> Option<i64> {
        self.http_status
    }

    pub fn duration_ms(&self) -> Option<i64> {
        self.duration_ms
    }

    pub fn reward_cents(&self) -> Option<i64> {
        self.reward_cents
    }

    pub fn source(&self) -> Option<CheckInSource> {
        self.source
    }

    pub fn is_open(&self) -> bool {
        self.finished_at.is_none() && self.outcome.is_none()
    }
}
