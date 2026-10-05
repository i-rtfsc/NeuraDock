use neuradock_domain::account::AccountRepository;
use neuradock_domain::check_in::Provider;
use neuradock_domain::shared::AccountId;
use std::sync::Arc;

use crate::application::services::BalanceService;

/// Task metadata for health monitoring
#[derive(Debug, Clone)]
pub(super) struct TaskMetadata {
    pub account_name: String,
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,
}

/// Configuration for spawning a check-in task
pub(super) struct CheckInTaskConfig {
    pub account_id: AccountId,
    pub account_name: String,
    pub hour: u8,
    pub minute: u8,
    pub provider: Provider,
    pub account_repo: Arc<dyn AccountRepository>,
    pub app_handle: tauri::AppHandle,
}

/// Configuration for watching a provider the app cannot post to.
pub(super) struct BalanceObservationTaskConfig {
    pub account_id: AccountId,
    pub account_name: String,
    pub balance_service: Arc<BalanceService>,
}

/// What the scheduler does for one provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScheduledTaskKind {
    /// Post a check-in at the account's configured time.
    CheckIn,
    /// Read the balance periodically. A web check-in then shows up as inferred
    /// evidence, which is the only way it reaches the ledger.
    ObserveBalance,
    /// Nothing to automate: the provider is disabled or unsupported.
    None,
}

/// Decide what the scheduler should do for a provider.
///
/// Observation-only providers are deliberately scheduled rather than skipped:
/// they are exactly the ones a user checks in for by hand on the website.
pub(super) fn scheduled_task_kind(provider: &Provider) -> ScheduledTaskKind {
    if provider.can_send_check_in_request() {
        ScheduledTaskKind::CheckIn
    } else if provider.check_in_mode().is_observe_only() {
        ScheduledTaskKind::ObserveBalance
    } else {
        ScheduledTaskKind::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neuradock_domain::check_in::{CheckInMode, ProviderConfig, DEFAULT_DAY_OFFSET_MINUTES};

    fn provider(
        sign_in_path: Option<&str>,
        supports_check_in: bool,
        check_in_bugged: bool,
        mode: CheckInMode,
    ) -> Provider {
        Provider::builtin(
            "test",
            ProviderConfig {
                name: "Test".to_string(),
                domain: "https://example.com".to_string(),
                login_path: "/login".to_string(),
                sign_in_path: sign_in_path.map(str::to_string),
                user_info_path: "/api/user/self".to_string(),
                token_api_path: None,
                models_path: None,
                api_user_key: "user".to_string(),
                bypass_method: None,
                supports_check_in,
                check_in_bugged,
                check_in_day_offset_minutes: DEFAULT_DAY_OFFSET_MINUTES,
                check_in_mode: mode,
            },
        )
    }

    #[test]
    fn an_api_provider_gets_a_check_in_task() {
        assert_eq!(
            scheduled_task_kind(&provider(
                Some("/api/user/sign_in"),
                true,
                false,
                CheckInMode::Api
            )),
            ScheduledTaskKind::CheckIn
        );
    }

    #[test]
    fn a_provider_the_app_cannot_post_to_is_observed_instead() {
        assert_eq!(
            scheduled_task_kind(&provider(None, true, true, CheckInMode::ObserveOnly)),
            ScheduledTaskKind::ObserveBalance,
            "agentrouter's shape: no endpoint, marked bugged, but its web check-in is still observable"
        );
    }

    #[test]
    fn a_disabled_provider_gets_no_task_and_no_log_noise() {
        assert_eq!(
            scheduled_task_kind(&provider(
                Some("/api/user/sign_in"),
                true,
                true,
                CheckInMode::Disabled
            )),
            ScheduledTaskKind::None
        );
        assert_eq!(
            scheduled_task_kind(&provider(None, false, false, CheckInMode::Disabled)),
            ScheduledTaskKind::None
        );
    }

    #[test]
    fn a_page_visit_provider_still_gets_a_check_in_task() {
        assert_eq!(
            scheduled_task_kind(&provider(Some("/signin"), true, false, CheckInMode::PageVisit)),
            ScheduledTaskKind::CheckIn
        );
    }
}
