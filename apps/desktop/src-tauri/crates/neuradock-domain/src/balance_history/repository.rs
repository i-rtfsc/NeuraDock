use async_trait::async_trait;
use chrono::NaiveDate;

use super::{BalanceHistoryDailySummary, BalanceHistoryRecord, CheckInRecord};
use crate::shared::{AccountId, DomainError};

#[async_trait]
pub trait BalanceHistoryRepository: Send + Sync {
    /// Persist a confirmed reward independently of balance refreshes.
    /// Returns true when inserted or when a legacy/unknown record was upgraded.
    async fn save_check_in(&self, record: &CheckInRecord) -> Result<bool, DomainError>;

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

    /// Find the latest balance record for an account on a specific date (UTC).
    async fn find_latest_by_account_id_on_date(
        &self,
        account_id: &AccountId,
        date: NaiveDate,
    ) -> Result<Option<BalanceHistoryRecord>, DomainError>;

    /// List daily summaries (grouped by date) for an account.
    async fn list_all_daily_summaries(
        &self,
        account_id: &AccountId,
    ) -> Result<Vec<BalanceHistoryDailySummary>, DomainError>;

    /// List daily summaries (grouped by date) for an account in a date range.
    async fn list_daily_summaries_in_range(
        &self,
        account_id: &AccountId,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<BalanceHistoryDailySummary>, DomainError>;

    /// Find a daily summary (grouped by date) for an account on a specific date.
    async fn find_daily_summary(
        &self,
        account_id: &AccountId,
        date: NaiveDate,
    ) -> Result<Option<BalanceHistoryDailySummary>, DomainError>;

    /// List distinct account IDs present in balance_history.
    async fn list_distinct_account_ids(&self) -> Result<Vec<AccountId>, DomainError>;
}
