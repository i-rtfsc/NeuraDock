use serde::{Deserialize, Serialize};
use specta::Type;

use super::BalanceDto;

/// One check-in attempt, as shown in the history view.
///
/// Attempts are operational records, so a failed or skipped run appears here
/// even though it produced no evidence.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CheckInAttemptDto {
    pub id: i64,
    pub account_id: String,
    pub account_name: String,
    pub provider_name: String,
    /// The provider's reset day this attempt belonged to (YYYY-MM-DD).
    pub quota_day: String,
    pub trigger: String,
    pub outcome: Option<String>,
    pub error_code: Option<String>,
    pub http_status: Option<i64>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub reward: Option<f64>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CheckInHistoryPageDto {
    pub total: i64,
    pub page: i32,
    pub page_size: i32,
    pub attempts: Vec<CheckInAttemptDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CheckInFailureDto {
    pub error_code: String,
    pub count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CheckInStatsDto {
    pub account_id: String,
    pub period_days: u32,
    pub start_date: String,
    pub end_date: String,
    pub attempts: i32,
    pub succeeded: i32,
    pub failed: i32,
    /// Runs the guard refused: not failures, but worth surfacing.
    pub skipped: i32,
    pub success_rate: f64,
    pub average_duration_ms: Option<i64>,
    /// Days with confirmed evidence in the period.
    pub confirmed_days: i32,
    /// Days whose evidence was inferred from a quota increase.
    pub inferred_days: i32,
    pub total_reward: f64,
    pub failures_by_code: Vec<CheckInFailureDto>,
}

/// Today's check-in state, so the UI can render the right affordance instead of
/// guessing from a timestamp.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CheckInStateDto {
    pub account_id: String,
    pub quota_day: String,
    /// `due`, `done`, `in_flight` or `backoff`.
    pub state: String,
    /// Present when `state` is `done`.
    pub confidence: Option<String>,
    /// Present when `state` is `backoff`.
    pub until: Option<String>,
    pub retryable_failures: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RunningJobDto {
    pub id: String,
    pub account_ids: Vec<String>,
    pub started_at: String,
    pub processed: u32,
    pub total: u32,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ExecuteCheckInResult {
    pub account_id: String,
    pub account_name: String,
    pub provider_id: String,
    pub success: bool,
    pub balance: Option<BalanceDto>,
    pub error: Option<String>,
    /// Stable token for what happened: `success`, `already_confirmed`,
    /// `skipped_guard`, `failed_retryable`, `failed_permanent`, `waf_blocked`
    /// or `cancelled`. `success` alone cannot distinguish "checked in" from
    /// "today was already done".
    pub outcome: String,
    /// Stable reason code when the run failed, so the UI can translate it
    /// instead of showing a raw message.
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BatchCheckInResult {
    pub total: i32,
    pub succeeded: i32,
    pub failed: i32,
    pub results: Vec<ExecuteCheckInResult>,
}
