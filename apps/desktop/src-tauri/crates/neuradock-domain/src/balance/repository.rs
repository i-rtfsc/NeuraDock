use async_trait::async_trait;

use super::Balance;
use crate::shared::{AccountId, DomainError};

/// Balance repository trait
#[async_trait]
pub trait BalanceRepository: Send + Sync {
    /// Save or update balance
    async fn save(&self, balance: &Balance) -> Result<(), DomainError>;

    /// Find balance by account ID
    async fn find_by_account_id(&self, account_id: &AccountId) -> Result<Option<Balance>, DomainError>;

    /// Delete balance by account ID
    async fn delete(&self, account_id: &AccountId) -> Result<(), DomainError>;

    /// Find all balances
    async fn find_all(&self) -> Result<Vec<Balance>, DomainError>;

    /// Find stale balances (not checked recently)
    async fn find_stale_balances(&self, hours_threshold: i64) -> Result<Vec<Balance>, DomainError>;
}
