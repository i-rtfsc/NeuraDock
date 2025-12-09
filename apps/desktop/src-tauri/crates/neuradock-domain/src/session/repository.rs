use async_trait::async_trait;

use super::Session;
use crate::shared::{AccountId, DomainError};

/// Session repository trait
#[async_trait]
pub trait SessionRepository: Send + Sync {
    /// Save or update a session
    async fn save(&self, session: &Session) -> Result<(), DomainError>;

    /// Find session by account ID
    async fn find_by_account_id(
        &self,
        account_id: &AccountId,
    ) -> Result<Option<Session>, DomainError>;

    /// Delete session by account ID
    async fn delete(&self, account_id: &AccountId) -> Result<(), DomainError>;

    /// Find all valid (non-expired) sessions
    async fn find_valid_sessions(&self) -> Result<Vec<Session>, DomainError>;
}
