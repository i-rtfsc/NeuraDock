use async_trait::async_trait;

use super::BalanceHistoryRecord;
use crate::shared::{AccountId, DomainError};

#[async_trait]
pub trait BalanceHistoryRepository: Send + Sync {
    /// Save (upsert) a balance history record.
    ///
    /// Callers can enforce domain rules such as "one record per day" by
    /// providing a deterministic `id`.
    async fn save(&self, record: &BalanceHistoryRecord) -> Result<(), DomainError>;

    /// Find the latest balance record for an account.
    async fn find_latest_by_account_id(
        &self,
        account_id: &AccountId,
    ) -> Result<Option<BalanceHistoryRecord>, DomainError>;
}

