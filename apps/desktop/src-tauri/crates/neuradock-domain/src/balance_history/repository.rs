use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};

use super::{
    AttemptCompletion, BalanceHistoryDailySummary, BalanceHistoryRecord, CheckInAttempt,
    CheckInRecord,
};
use crate::shared::{AccountId, DomainError};

/// The check-in ledger.
///
/// Two things live here because they are read together and must be written
/// atomically: balance snapshots (what the provider reported over time) and
/// check-in evidence plus attempts (what the app knows happened). The daily
/// summary query joins them, which is why they share one interface.
#[async_trait]
pub trait BalanceHistoryRepository: Send + Sync {
    /// Persist check-in evidence and refresh the account's cached check-in
    /// timestamps in the same transaction.
    ///
    /// Returns true when a row was inserted or promoted. A row that is already
    /// at least as trustworthy is left untouched and returns false, so callers
    /// can tell "recorded" from "someone better already said so".
    async fn save_check_in(&self, record: &CheckInRecord) -> Result<bool, DomainError>;

    /// Save (upsert) a balance history record.
    ///
    /// Callers can enforce domain rules such as "one record per day" by
    /// providing a deterministic `id`.
    async fn save(&self, record: &BalanceHistoryRecord) -> Result<(), DomainError>;

    /// Persist one balance observation, the evidence it may imply, and the
    /// account's cached timestamps in a single transaction.
    ///
    /// `inferred` is the evidence the observation justifies (a quota increase
    /// over the previous quota day), or `None` when the observation proves
    /// nothing. Returns whether that evidence was actually recorded.
    async fn save_balance_observation(
        &self,
        snapshot: &BalanceHistoryRecord,
        inferred: Option<&CheckInRecord>,
    ) -> Result<bool, DomainError>;

    /// Find the evidence row for one quota day, if the day is satisfied.
    async fn find_evidence(
        &self,
        account_id: &AccountId,
        quota_day: NaiveDate,
    ) -> Result<Option<CheckInRecord>, DomainError>;

    /// List evidence rows for a quota-day range, oldest first.
    async fn list_evidence_in_range(
        &self,
        account_id: &AccountId,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<CheckInRecord>, DomainError>;

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

    /// List daily summaries (grouped by quota day) for an account.
    async fn list_all_daily_summaries(
        &self,
        account_id: &AccountId,
    ) -> Result<Vec<BalanceHistoryDailySummary>, DomainError>;

    /// List daily summaries (grouped by quota day) for an account in a range.
    async fn list_daily_summaries_in_range(
        &self,
        account_id: &AccountId,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<BalanceHistoryDailySummary>, DomainError>;

    /// Find a daily summary (grouped by quota day) for an account.
    async fn find_daily_summary(
        &self,
        account_id: &AccountId,
        date: NaiveDate,
    ) -> Result<Option<BalanceHistoryDailySummary>, DomainError>;

    /// List distinct account IDs present in the ledger.
    async fn list_distinct_account_ids(&self) -> Result<Vec<AccountId>, DomainError>;

    /// Open an attempt. Returns its id, which the caller closes on completion.
    async fn start_attempt(&self, attempt: &CheckInAttempt) -> Result<i64, DomainError>;

    /// Close an attempt with its outcome.
    async fn finish_attempt(
        &self,
        attempt_id: i64,
        completion: &AttemptCompletion,
    ) -> Result<(), DomainError>;

    /// Find an attempt for the quota day that started after `started_after` and
    /// has not been closed yet -- the single-flight lease.
    async fn find_open_attempt(
        &self,
        account_id: &AccountId,
        quota_day: NaiveDate,
        started_after: DateTime<Utc>,
    ) -> Result<Option<CheckInAttempt>, DomainError>;

    /// List attempts for one quota day, oldest first. The state machine derives
    /// its backoff from the tail of this list.
    async fn list_attempts_on_day(
        &self,
        account_id: &AccountId,
        quota_day: NaiveDate,
    ) -> Result<Vec<CheckInAttempt>, DomainError>;

    /// List attempts newest first, optionally narrowed to one account, with
    /// pagination. Backs the check-in history view.
    async fn list_attempts(
        &self,
        account_id: Option<&AccountId>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CheckInAttempt>, DomainError>;

    /// Count attempts, optionally narrowed to one account, so the history view
    /// can page without loading everything.
    async fn count_attempts(&self, account_id: Option<&AccountId>) -> Result<i64, DomainError>;

    /// List attempts whose quota day falls in a range, oldest first. Backs the
    /// statistics view, which aggregates by day.
    async fn list_attempts_in_range(
        &self,
        account_id: &AccountId,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<CheckInAttempt>, DomainError>;

    /// Recompute the account's cached check-in timestamps from the ledger.
    /// Used when evidence is written outside the observation path, and by the
    /// repair command that heals drift.
    async fn refresh_check_in_cache(&self, account_id: &AccountId) -> Result<(), DomainError>;
}
