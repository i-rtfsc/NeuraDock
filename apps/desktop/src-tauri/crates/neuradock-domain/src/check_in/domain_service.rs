use crate::account::Account;
use crate::check_in::Provider;
use crate::shared::DomainError;

/// Domain service for check-in business rules
/// Contains pure domain logic without infrastructure dependencies
pub struct CheckInDomainService;

impl CheckInDomainService {
    /// Minimum interval between check-ins (23 hours)
    /// This prevents abuse and respects provider's rate limits
    const MIN_CHECK_IN_INTERVAL_HOURS: i64 = 23;

    /// Validate if account can perform check-in
    pub fn can_check_in(account: &Account) -> Result<(), DomainError> {
        if !account.is_enabled() {
            return Err(DomainError::Validation(
                "Account is disabled and cannot perform check-in".to_string(),
            ));
        }

        // Check if check-in is too frequent
        if let Some(last_check_in) = account.last_check_in() {
            let now = chrono::Utc::now();
            let elapsed = now.signed_duration_since(last_check_in);
            let hours_since_last = elapsed.num_hours();

            if hours_since_last < Self::MIN_CHECK_IN_INTERVAL_HOURS {
                let hours_remaining = Self::MIN_CHECK_IN_INTERVAL_HOURS - hours_since_last;
                return Err(DomainError::Validation(format!(
                    "Check-in too frequent. Please wait {} hour(s) before next check-in. Last check-in: {}",
                    hours_remaining,
                    last_check_in.format("%Y-%m-%d %H:%M:%S UTC")
                )));
            }
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

        Ok(())
    }

    /// Calculate check-in reward based on provider rules
    pub fn calculate_reward(_provider: &Provider, is_consecutive: bool) -> f64 {
        // Base reward logic - can be extended based on provider configuration
        let base_reward = 1.0; // Default base reward

        if is_consecutive {
            // Bonus for consecutive check-ins
            base_reward * 1.5
        } else {
            base_reward
        }
    }

    /// Determine if session needs refresh based on business rules
    pub fn should_refresh_session(account: &Account) -> bool {
        // Session is invalid or will expire soon (within 1 hour)
        !account.is_session_valid() || {
            if let Some(expires_at) = account.session_expires_at() {
                let now = chrono::Utc::now();
                let one_hour_later = now + chrono::Duration::hours(1);
                expires_at < one_hour_later
            } else {
                true
            }
        }
    }

    /// Determine if balance check is needed
    pub fn should_check_balance(account: &Account, hours_threshold: i64) -> bool {
        account.is_balance_stale(hours_threshold)
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
        use crate::check_in::ProviderConfig;

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

    #[test]
    fn test_cannot_check_in_too_frequent() {
        use crate::account::Credentials;
        use crate::shared::{AccountId, ProviderId};
        use chrono::{Duration, Utc};

        let mut cookies = HashMap::new();
        cookies.insert("session".to_string(), "test_session".to_string());

        // Create account with recent check-in (2 hours ago)
        let last_check_in = Utc::now() - Duration::hours(2);
        let account = Account::builder(
            AccountId::new(),
            "Test Account".to_string(),
            ProviderId::new(),
            Credentials::new(cookies, "test@user".to_string()),
        )
        .last_check_in(Some(last_check_in))
        .created_at(Utc::now() - Duration::days(1))
        .build();

        let result = CheckInDomainService::can_check_in(&account);
        assert!(result.is_err());

        match result {
            Err(DomainError::Validation(msg)) => {
                assert!(msg.contains("too frequent"));
                assert!(msg.contains("21 hour")); // 23 - 2 = 21
            }
            _ => panic!("Expected Validation error for frequency check"),
        }
    }

    #[test]
    fn test_can_check_in_after_interval() {
        use crate::account::Credentials;
        use crate::shared::{AccountId, ProviderId};
        use chrono::{Duration, Utc};

        let mut cookies = HashMap::new();
        cookies.insert("session".to_string(), "test_session".to_string());

        // Create account with old check-in (24 hours ago - allowed)
        let last_check_in = Utc::now() - Duration::hours(24);
        let account = Account::builder(
            AccountId::new(),
            "Test Account".to_string(),
            ProviderId::new(),
            Credentials::new(cookies, "test@user".to_string()),
        )
        .last_check_in(Some(last_check_in))
        .created_at(Utc::now() - Duration::days(1))
        .build();

        let result = CheckInDomainService::can_check_in(&account);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_provider() {
        let provider = create_test_provider();
        assert!(CheckInDomainService::validate_provider(&provider).is_ok());
    }

    #[test]
    fn test_calculate_reward() {
        let provider = create_test_provider();

        let reward = CheckInDomainService::calculate_reward(&provider, false);
        assert_eq!(reward, 1.0); // Default base reward

        let consecutive_reward = CheckInDomainService::calculate_reward(&provider, true);
        assert_eq!(consecutive_reward, 1.5); // 1.0 * 1.5
    }

    #[test]
    fn test_should_refresh_session_invalid() {
        let account = create_test_account();
        // New account has no session
        assert!(CheckInDomainService::should_refresh_session(&account));
    }

    #[test]
    fn test_should_check_balance() {
        let account = create_test_account();
        // New account has stale balance
        assert!(CheckInDomainService::should_check_balance(&account, 24));
    }
}
