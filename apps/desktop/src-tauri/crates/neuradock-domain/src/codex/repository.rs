use async_trait::async_trait;

use super::aggregate::{CodexAccount, CodexAccountId};
use crate::shared::DomainError;

#[async_trait]
pub trait CodexAccountRepository: Send + Sync {
    async fn save(&self, account: &CodexAccount) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: &CodexAccountId) -> Result<Option<CodexAccount>, DomainError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<CodexAccount>, DomainError>;
    async fn find_all(&self) -> Result<Vec<CodexAccount>, DomainError>;
    async fn delete(&self, id: &CodexAccountId) -> Result<(), DomainError>;
    async fn count(&self) -> Result<i64, DomainError>;
}
