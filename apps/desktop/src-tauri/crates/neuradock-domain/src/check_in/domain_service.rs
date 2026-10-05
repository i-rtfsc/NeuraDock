use crate::account::Account;
use crate::check_in::{CheckInMode, Provider};
use crate::shared::DomainError;

/// Domain service for check-in business rules
/// Contains pure domain logic without infrastructure dependencies
pub struct CheckInDomainService;

impl CheckInDomainService {
    /// Whether this account may attempt a check-in at all.
    ///
    /// Retry pacing is deliberately not decided here. "Is today already
    /// satisfied, in flight, or backing off?" is answered from the ledger by
    /// [`evaluate_check_in_state`](crate::check_in::evaluate_check_in_state),
    /// because elapsed time cannot tell a check-in from a balance refresh, and
    /// a 24h interval cannot represent a provider's reset day.
    pub fn can_check_in(account: &Account) -> Result<(), DomainError> {
        if !account.is_enabled() {
            return Err(DomainError::Validation(
                "Account is disabled and cannot perform check-in".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate provider configuration
    pub fn validate_provider(provider: &Provider) -> Result<(), DomainError> {
        // Provider is valid as long as it has basic configuration
        // sign_in_url is optional - some providers (like AgentRouter) auto check-in when querying user info
        if provider.domain().is_empty() {
            return Err(DomainError::ProviderNotFound(
                "Provider domain is not configured".to_string(),
            ));
        }

        if provider.user_info_url().is_empty() {
            return Err(DomainError::ProviderNotFound(
                "Provider user info URL is not configured".to_string(),
            ));
        }

        if !provider.supports_check_in() {
            return Err(DomainError::Validation(
                "This provider does not support automated check-in".to_string(),
            ));
        }

        if provider.check_in_bugged() {
            return Err(DomainError::Validation(
                "Check-in temporarily disabled due to known issue".to_string(),
            ));
        }

        match provider.check_in_mode() {
            CheckInMode::Api | CheckInMode::PageVisit => Ok(()),
            CheckInMode::ObserveOnly => Err(DomainError::Validation(
                "This provider only supports a web check-in; record it manually or let the next balance refresh detect it".to_string(),
            )),
            CheckInMode::Disabled => Err(DomainError::Validation(
                "This provider cannot be checked in by the app".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::Credentials;
    use crate::shared::ProviderId;
    use std::collections::HashMap;

    fn create_test_account() -> Account {
        let mut cookies = HashMap::new();
        cookies.insert("session".to_string(), "test_session".to_string());

        Account::new(
            "Test Account".to_string(),
            ProviderId::new(),
            Credentials::new(cookies, "test@user".to_string()),
        )
        .unwrap()
    }

    fn create_test_provider() -> Provider {
        use crate::check_in::{CheckInMode, ProviderConfig, DEFAULT_DAY_OFFSET_MINUTES};

        Provider::new(ProviderConfig {
            name: "Test Provider".to_string(),
            domain: "https://example.com".to_string(),
            login_path: "/login".to_string(),
            sign_in_path: Some("/checkin".to_string()),
            user_info_path: "/userinfo".to_string(),
            token_api_path: Some("/token".to_string()),
            models_path: Some("/models".to_string()),
            api_user_key: "user".to_string(),
            bypass_method: None,
            supports_check_in: true,
            check_in_bugged: false,
            check_in_day_offset_minutes: DEFAULT_DAY_OFFSET_MINUTES,
            check_in_mode: CheckInMode::Api,
        })
    }

    #[test]
    fn test_can_check_in_enabled_account() {
        let account = create_test_account();
        assert!(CheckInDomainService::can_check_in(&account).is_ok());
    }

    #[test]
    fn test_cannot_check_in_disabled_account() {
        let mut account = create_test_account();
        account.toggle(false);

        let result = CheckInDomainService::can_check_in(&account);
        assert!(result.is_err());

        match result {
            Err(DomainError::Validation(msg)) => {
                assert!(msg.contains("disabled"));
            }
            _ => panic!("Expected Validation error"),
        }
    }

    /// The exact shape that used to turn daily automation into every-other-day:
    /// the scheduler fires at the same wall-clock time each day, but the
    /// previous run's timestamp was stamped after the HTTP round trip. Retry
    /// pacing now comes from `evaluate_check_in_state`, which is covered there.
    #[test]
    fn test_validate_provider() {
        let provider = create_test_provider();
        assert!(CheckInDomainService::validate_provider(&provider).is_ok());
    }

    #[test]
    fn observe_only_providers_are_refused_an_automated_check_in() {
        use crate::check_in::{CheckInMode, ProviderConfig, DEFAULT_DAY_OFFSET_MINUTES};

        let provider = Provider::new(ProviderConfig {
            name: "Web Only".to_string(),
            domain: "https://example.com".to_string(),
            login_path: "/login".to_string(),
            sign_in_path: None,
            user_info_path: "/userinfo".to_string(),
            token_api_path: None,
            models_path: None,
            api_user_key: "user".to_string(),
            bypass_method: None,
            supports_check_in: true,
            check_in_bugged: false,
            check_in_day_offset_minutes: DEFAULT_DAY_OFFSET_MINUTES,
            check_in_mode: CheckInMode::ObserveOnly,
        });

        let error = CheckInDomainService::validate_provider(&provider)
            .expect_err("an observe-only provider cannot be posted to");
        assert!(error.to_string().contains("web check-in"));
    }
}
