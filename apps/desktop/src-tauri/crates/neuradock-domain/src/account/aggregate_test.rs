#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::shared::{DomainError, ProviderId};
    use chrono::Utc;
    use std::collections::HashMap;

    fn create_test_credentials() -> Credentials {
        let mut cookies = HashMap::new();
        cookies.insert("session".to_string(), "test_token".to_string());
        Credentials::new(cookies, "test@user".to_string())
    }

    #[test]
    fn test_create_account_with_valid_data() {
        let credentials = create_test_credentials();

        let result = Account::new(
            "Test Account".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        );

        assert!(result.is_ok());
        let account = result.unwrap();
        assert_eq!(account.name(), "Test Account");
        assert_eq!(account.provider_id().as_str(), "anyrouter");
        assert!(account.is_enabled());
        assert!(!account.auto_checkin_enabled());
        assert_eq!(account.auto_checkin_hour(), 9);
        assert_eq!(account.auto_checkin_minute(), 0);
    }

    #[test]
    fn test_create_account_with_empty_name_fails() {
        let credentials = create_test_credentials();

        let result = Account::new(
            "".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        );

        assert!(result.is_err());
        match result {
            Err(DomainError::Validation(msg)) => {
                assert!(msg.contains("cannot be empty"));
            }
            _ => panic!("Expected validation error for empty name"),
        }
    }

    #[test]
    fn test_create_account_with_whitespace_name_fails() {
        let credentials = create_test_credentials();

        let result = Account::new(
            "   ".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_create_account_with_invalid_credentials_fails() {
        let empty_cookies = HashMap::new();
        let credentials = Credentials::new(empty_cookies, "test@user".to_string());

        let result = Account::new(
            "Test Account".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        );

        assert!(result.is_err());
        match result {
            Err(DomainError::InvalidCredentials(_)) => {
                // Expected
            }
            _ => panic!("Expected invalid credentials error"),
        }
    }

    #[test]
    fn test_update_name_with_valid_name() {
        let credentials = create_test_credentials();
        let mut account = Account::new(
            "Original Name".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        )
        .unwrap();

        let result = account.update_name("Updated Name".to_string());

        assert!(result.is_ok());
        assert_eq!(account.name(), "Updated Name");
    }

    #[test]
    fn test_update_name_with_empty_name_fails() {
        let credentials = create_test_credentials();
        let mut account = Account::new(
            "Original Name".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        )
        .unwrap();

        let result = account.update_name("".to_string());

        assert!(result.is_err());
        assert_eq!(account.name(), "Original Name"); // Name should not change
    }

    #[test]
    fn test_update_name_trims_whitespace() {
        let credentials = create_test_credentials();
        let mut account = Account::new(
            "Original Name".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        )
        .unwrap();

        let result = account.update_name("  Trimmed Name  ".to_string());

        assert!(result.is_ok());
        assert_eq!(account.name(), "Trimmed Name");
    }

    #[test]
    fn test_toggle_account() {
        let credentials = create_test_credentials();
        let mut account = Account::new(
            "Test Account".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        )
        .unwrap();

        assert!(account.is_enabled());

        account.toggle(false);
        assert!(!account.is_enabled());

        account.toggle(true);
        assert!(account.is_enabled());
    }

    #[test]
    fn test_record_check_in() {
        let credentials = create_test_credentials();
        let mut account = Account::new(
            "Test Account".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        )
        .unwrap();

        assert!(account.last_check_in().is_none());

        let before = Utc::now();
        account.record_check_in();
        let after = Utc::now();

        assert!(account.last_check_in().is_some());
        let last_check_in = account.last_check_in().unwrap();
        assert!(last_check_in >= before && last_check_in <= after);
    }

    #[test]
    fn test_update_auto_checkin_with_valid_values() {
        let credentials = create_test_credentials();
        let mut account = Account::new(
            "Test Account".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        )
        .unwrap();

        let result = account.update_auto_checkin(true, 10, 30);

        assert!(result.is_ok());
        assert!(account.auto_checkin_enabled());
        assert_eq!(account.auto_checkin_hour(), 10);
        assert_eq!(account.auto_checkin_minute(), 30);
    }

    #[test]
    fn test_update_auto_checkin_with_invalid_hour_fails() {
        let credentials = create_test_credentials();
        let mut account = Account::new(
            "Test Account".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        )
        .unwrap();

        let result = account.update_auto_checkin(true, 24, 0);

        assert!(result.is_err());
        match result {
            Err(DomainError::Validation(msg)) => {
                assert!(msg.contains("Hour"));
                assert!(msg.contains("0 and 23"));
            }
            _ => panic!("Expected validation error for invalid hour"),
        }
    }

    #[test]
    fn test_update_auto_checkin_with_invalid_minute_fails() {
        let credentials = create_test_credentials();
        let mut account = Account::new(
            "Test Account".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        )
        .unwrap();

        let result = account.update_auto_checkin(true, 9, 60);

        assert!(result.is_err());
        match result {
            Err(DomainError::Validation(msg)) => {
                assert!(msg.contains("Minute"));
                assert!(msg.contains("0 and 59"));
            }
            _ => panic!("Expected validation error for invalid minute"),
        }
    }

    #[test]
    fn test_update_session() {
        let credentials = create_test_credentials();
        let mut account = Account::new(
            "Test Account".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        )
        .unwrap();

        assert!(account.session_token().is_none());
        assert!(account.session_expires_at().is_none());
        assert!(!account.is_session_valid());

        let token = "session_token_123".to_string();
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        account.update_session(token.clone(), expires_at);

        assert_eq!(account.session_token(), Some(&token));
        assert_eq!(account.session_expires_at(), Some(expires_at));
        assert!(account.is_session_valid());
        assert!(account.last_login_at().is_some());
    }

    #[test]
    fn test_clear_session() {
        let credentials = create_test_credentials();
        let mut account = Account::new(
            "Test Account".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        )
        .unwrap();

        // Set session first
        let expires_at = Utc::now() + chrono::Duration::hours(1);
        account.update_session("token".to_string(), expires_at);
        assert!(account.is_session_valid());

        // Clear session
        account.clear_session();

        assert!(account.session_token().is_none());
        assert!(account.session_expires_at().is_none());
        assert!(!account.is_session_valid());
    }

    #[test]
    fn test_is_session_valid_with_expired_session() {
        let credentials = create_test_credentials();
        let mut account = Account::new(
            "Test Account".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        )
        .unwrap();

        // Set expired session
        let expired = Utc::now() - chrono::Duration::hours(1);
        account.update_session("token".to_string(), expired);

        assert!(!account.is_session_valid());
    }

    #[test]
    fn test_update_balance() {
        let credentials = create_test_credentials();
        let mut account = Account::new(
            "Test Account".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        )
        .unwrap();

        assert!(account.current_balance().is_none());
        assert!(account.total_consumed().is_none());
        assert!(account.total_income().is_none());

        account.update_balance(100.0, 50.0, 150.0);

        assert_eq!(account.current_balance(), Some(100.0));
        assert_eq!(account.total_consumed(), Some(50.0));
        assert_eq!(account.total_income(), Some(150.0));
        assert!(account.last_balance_check_at().is_some());
    }

    #[test]
    fn test_is_balance_stale_with_no_check() {
        let credentials = create_test_credentials();
        let account = Account::new(
            "Test Account".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        )
        .unwrap();

        // No balance check yet, should be stale
        assert!(account.is_balance_stale(24));
    }

    #[test]
    fn test_is_balance_stale_with_recent_check() {
        let credentials = create_test_credentials();
        let mut account = Account::new(
            "Test Account".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        )
        .unwrap();

        // Update balance (sets last_balance_check_at to now)
        account.update_balance(100.0, 50.0, 150.0);

        // Should not be stale (checked just now)
        assert!(!account.is_balance_stale(24));
    }

    #[test]
    fn test_update_credentials_with_valid_credentials() {
        let credentials = create_test_credentials();
        let mut account = Account::new(
            "Test Account".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        )
        .unwrap();

        let mut new_cookies = HashMap::new();
        new_cookies.insert("session".to_string(), "new_token".to_string());
        let new_credentials = Credentials::new(new_cookies, "new@user".to_string());

        let result = account.update_credentials(new_credentials);

        assert!(result.is_ok());
        assert_eq!(account.credentials().api_user(), "new@user");
    }

    #[test]
    fn test_update_credentials_with_invalid_credentials_fails() {
        let credentials = create_test_credentials();
        let mut account = Account::new(
            "Test Account".to_string(),
            ProviderId::from_string("anyrouter"),
            credentials,
        )
        .unwrap();

        let empty_cookies = HashMap::new();
        let invalid_credentials = Credentials::new(empty_cookies, "test@user".to_string());

        let result = account.update_credentials(invalid_credentials);

        assert!(result.is_err());
    }
}
