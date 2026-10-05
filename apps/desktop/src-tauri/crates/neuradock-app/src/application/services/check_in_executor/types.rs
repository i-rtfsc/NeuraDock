use neuradock_domain::balance_history::CheckInOutcome;
use neuradock_infrastructure::http::UserInfo;

/// Check-in result for a single account.
#[derive(Debug, Clone)]
pub struct AccountCheckInResult {
    pub account_name: String,
    pub success: bool,
    pub message: String,
    pub user_info: Option<UserInfo>,
    /// What actually happened, so callers can tell "checked in" from "was
    /// already done" from "failed" without parsing a message.
    pub outcome: CheckInOutcome,
}

impl AccountCheckInResult {
    /// The quota day was already satisfied; nothing was sent.
    pub fn already_done(account_name: String) -> Self {
        Self {
            account_name,
            success: true,
            message: "Already checked in for this quota day".to_string(),
            user_info: None,
            outcome: CheckInOutcome::AlreadyConfirmed,
        }
    }

    /// The guard refused the attempt (in flight, or backing off).
    pub fn skipped(account_name: String, message: impl Into<String>) -> Self {
        Self {
            account_name,
            success: false,
            message: message.into(),
            user_info: None,
            outcome: CheckInOutcome::SkippedGuard,
        }
    }
}
