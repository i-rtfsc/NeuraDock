mod repository;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::{AccountId, DomainError};

pub use repository::SessionRepository;

/// Session value object
/// Represents user authentication session for an account
///
/// This is a separate value object instead of being part of Account aggregate
/// because session has different lifecycle and change frequency than account credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    account_id: AccountId,
    token: String,
    expires_at: DateTime<Utc>,
    last_login_at: DateTime<Utc>,
}

impl Session {
    /// Create a new session
    pub fn new(
        account_id: AccountId,
        token: String,
        expires_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if token.is_empty() {
            return Err(DomainError::Validation(
                "Session token cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            account_id,
            token,
            expires_at,
            last_login_at: Utc::now(),
        })
    }

    /// Restore session from persistence
    pub fn restore(
        account_id: AccountId,
        token: String,
        expires_at: DateTime<Utc>,
        last_login_at: DateTime<Utc>,
    ) -> Self {
        Self {
            account_id,
            token,
            expires_at,
            last_login_at,
        }
    }

    /// Check if session is still valid
    pub fn is_valid(&self) -> bool {
        Utc::now() < self.expires_at
    }

    /// Check if session will expire soon (within given hours)
    pub fn expires_soon(&self, hours: i64) -> bool {
        let threshold = Utc::now() + Duration::hours(hours);
        self.expires_at < threshold
    }

    /// Update session token and expiration
    pub fn update(&mut self, token: String, expires_at: DateTime<Utc>) -> Result<(), DomainError> {
        if token.is_empty() {
            return Err(DomainError::Validation(
                "Session token cannot be empty".to_string(),
            ));
        }

        self.token = token;
        self.expires_at = expires_at;
        self.last_login_at = Utc::now();

        Ok(())
    }

    /// Mark session as expired
    pub fn expire(&mut self) {
        self.expires_at = Utc::now() - Duration::seconds(1);
    }

    // Getters
    pub fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    pub fn last_login_at(&self) -> DateTime<Utc> {
        self.last_login_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_session() -> Session {
        let account_id = AccountId::new();
        let token = "test_token_123".to_string();
        let expires_at = Utc::now() + Duration::hours(24);

        Session::new(account_id, token, expires_at).unwrap()
    }

    #[test]
    fn test_create_session() {
        let session = create_test_session();
        assert!(session.is_valid());
        assert_eq!(session.token(), "test_token_123");
    }

    #[test]
    fn test_empty_token_rejected() {
        let account_id = AccountId::new();
        let expires_at = Utc::now() + Duration::hours(24);

        let result = Session::new(account_id, "".to_string(), expires_at);
        assert!(result.is_err());
    }

    #[test]
    fn test_session_expiration() {
        let account_id = AccountId::new();
        let token = "test_token".to_string();
        let expires_at = Utc::now() - Duration::hours(1); // Expired 1 hour ago

        let session = Session::new(account_id, token, expires_at).unwrap();
        assert!(!session.is_valid());
    }

    #[test]
    fn test_session_expires_soon() {
        let account_id = AccountId::new();
        let token = "test_token".to_string();
        let expires_at = Utc::now() + Duration::hours(2); // Expires in 2 hours

        let session = Session::new(account_id, token, expires_at).unwrap();
        assert!(session.expires_soon(3)); // Will expire within 3 hours
        assert!(!session.expires_soon(1)); // Won't expire within 1 hour
    }

    #[test]
    fn test_update_session() {
        let mut session = create_test_session();
        let new_token = "new_token_456".to_string();
        let new_expires_at = Utc::now() + Duration::hours(48);

        session.update(new_token.clone(), new_expires_at).unwrap();
        assert_eq!(session.token(), &new_token);
        assert_eq!(session.expires_at(), new_expires_at);
    }

    #[test]
    fn test_expire_session() {
        let mut session = create_test_session();
        assert!(session.is_valid());

        session.expire();
        assert!(!session.is_valid());
    }
}
