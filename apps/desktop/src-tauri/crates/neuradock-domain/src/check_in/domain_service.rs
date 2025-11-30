use crate::account::Account;
use crate::check_in::Provider;
use crate::shared::DomainError;

/// Domain service for check-in business rules
/// Contains pure domain logic without infrastructure dependencies
pub struct CheckInDomainService;

impl CheckInDomainService {
    /// Validate if account can perform check-in
    pub fn can_check_in(account: &Account) -> Result<(), DomainError> {
        if !account.is_enabled() {
            return Err(DomainError::Validation(
                "Account is disabled and cannot perform check-in".to_string(),
            ));
        }

        // Additional domain rules can be added here
        // For example: check last check-in time, rate limiting, etc.

        Ok(())
    }

    /// Validate provider configuration
    pub fn validate_provider(provider: &Provider) -> Result<(), DomainError> {
        if provider.sign_in_url().is_none() || provider.sign_in_url().unwrap().is_empty() {
            return Err(DomainError::ProviderNotFound(
                "Provider check-in URL is not configured".to_string(),
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
        Provider::new(
            "Test Provider".to_string(),
            "https://example.com".to_string(),
            "/login".to_string(),
            Some("/checkin".to_string()),
            "/userinfo".to_string(),
            "user".to_string(),
            None,
        )
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
