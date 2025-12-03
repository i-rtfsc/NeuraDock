use async_trait::async_trait;

use crate::shared::{AccountId, DomainError};
use super::aggregate::{ApiToken, TokenId};

#[async_trait]
pub trait TokenRepository: Send + Sync {
    async fn save(&self, token: &ApiToken) -> Result<(), DomainError>;
    async fn save_batch(&self, tokens: Vec<ApiToken>) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: &TokenId) -> Result<Option<ApiToken>, DomainError>;
    async fn find_by_account(&self, account_id: &AccountId) -> Result<Vec<ApiToken>, DomainError>;
    async fn delete_by_account(&self, account_id: &AccountId) -> Result<(), DomainError>;
}
