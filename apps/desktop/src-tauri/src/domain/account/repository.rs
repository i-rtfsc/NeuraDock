use super::Account;
use crate::domain::shared::{AccountId, DomainError};
use async_trait::async_trait;

#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn save(&self, account: &Account) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: &AccountId) -> Result<Option<Account>, DomainError>;
    async fn find_all(&self) -> Result<Vec<Account>, DomainError>;
    async fn find_enabled(&self) -> Result<Vec<Account>, DomainError>;
    async fn delete(&self, id: &AccountId) -> Result<(), DomainError>;
}
