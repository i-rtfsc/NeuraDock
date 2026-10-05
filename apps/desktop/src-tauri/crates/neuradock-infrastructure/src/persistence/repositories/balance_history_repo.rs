use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;

use crate::persistence::SqliteRepositoryBase;
use neuradock_domain::balance_history::{
    BalanceHistoryDailySummary, BalanceHistoryRecord, BalanceHistoryRepository, CheckInRecord,
};
use neuradock_domain::shared::{AccountId, DomainError};

#[derive(FromRow)]
struct BalanceHistoryRow {
    id: String,
    account_id: String,
    current_balance: f64,
    total_consumed: f64,
    total_quota: f64,
    recorded_at: DateTime<Utc>,
}

impl BalanceHistoryRow {
    fn into_record(self) -> BalanceHistoryRecord {
        BalanceHistoryRecord::restore(
            self.id,
            AccountId::from_string(&self.account_id),
            self.current_balance,
            self.total_consumed,
            self.total_quota,
            self.recorded_at,
        )
    }
}

#[derive(FromRow)]
struct DailySummaryRow {
    check_in_date: String,
    daily_total_quota: f64,
    daily_balance: f64,
    daily_consumed: f64,
    check_in_reward: Option<f64>,
    check_in_source: Option<String>,
}

impl DailySummaryRow {
    fn try_into_summary(self) -> Result<BalanceHistoryDailySummary, DomainError> {
        let date = NaiveDate::parse_from_str(&self.check_in_date, "%Y-%m-%d").map_err(|e| {
            DomainError::Validation(format!(
                "Invalid check_in_date: {} ({})",
                self.check_in_date, e
            ))
        })?;

        Ok(BalanceHistoryDailySummary::restore(
            date,
            self.daily_total_quota,
            self.daily_balance,
            self.daily_consumed,
        )
        .with_check_in(self.check_in_reward, self.check_in_source))
    }
}

// Read all balance fields from the same snapshot. A reward also remains
// visible when the post-check-in balance request failed or was delayed.
const DAILY_SUMMARY_QUERY: &str = r#"
    WITH days AS (
        SELECT DATE(recorded_at) AS day FROM balance_history WHERE account_id = ?1
        UNION
        SELECT check_in_date AS day FROM check_in_records WHERE account_id = ?1
    )
    SELECT days.day AS check_in_date,
           COALESCE(b.total_quota, 0.0) AS daily_total_quota,
           COALESCE(b.current_balance, 0.0) AS daily_balance,
           COALESCE(b.total_consumed, 0.0) AS daily_consumed,
           c.reward_cents / 100.0 AS check_in_reward,
           c.source AS check_in_source
    FROM days
    LEFT JOIN balance_history b ON b.id = (
        SELECT id FROM balance_history
        WHERE account_id = ?1 AND DATE(recorded_at) <= days.day
        ORDER BY recorded_at DESC, id DESC LIMIT 1
    )
    LEFT JOIN check_in_records c ON c.account_id = ?1 AND c.check_in_date = days.day
"#;

pub struct SqliteBalanceHistoryRepository {
    base: SqliteRepositoryBase,
}

impl SqliteBalanceHistoryRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self {
            base: SqliteRepositoryBase::new(pool),
        }
    }
}

#[async_trait]
impl BalanceHistoryRepository for SqliteBalanceHistoryRepository {
    async fn save_check_in(&self, record: &CheckInRecord) -> Result<bool, DomainError> {
        let result = sqlx::query(r#"
            INSERT INTO check_in_records (account_id, check_in_date, reward_cents, source, recorded_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(account_id, check_in_date) DO UPDATE SET
                reward_cents = excluded.reward_cents,
                source = excluded.source,
                recorded_at = excluded.recorded_at
            WHERE (check_in_records.source = 'legacy_estimate' AND excluded.source != 'legacy_estimate')
               OR (check_in_records.source = 'confirmed_unknown' AND excluded.reward_cents IS NOT NULL)
               OR (check_in_records.source = 'provider_rule' AND excluded.source = 'provider_response')
               OR excluded.source = 'manual'
        "#)
        .bind(record.account_id.as_str())
        .bind(record.check_in_date.format("%Y-%m-%d").to_string())
        .bind(record.reward_cents)
        .bind(&record.source)
        .bind(record.recorded_at)
        .execute(self.base.pool())
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Save check-in reward: {e}")))?;
        Ok(result.rows_affected() > 0)
    }

    async fn save(&self, record: &BalanceHistoryRecord) -> Result<(), DomainError> {
        let query = r#"
            INSERT OR REPLACE INTO balance_history (
                id,
                account_id,
                current_balance,
                total_consumed,
                total_quota,
                recorded_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#;

        self.base
            .execute(
                sqlx::query(query)
                    .bind(record.id())
                    .bind(record.account_id().as_str())
                    .bind(record.current_balance())
                    .bind(record.total_consumed())
                    .bind(record.total_quota())
                    .bind(record.recorded_at()),
                "Save balance history",
            )
            .await?;

        Ok(())
    }

    async fn find_latest_by_account_id(
        &self,
        account_id: &AccountId,
    ) -> Result<Option<BalanceHistoryRecord>, DomainError> {
        let query = r#"
            SELECT
                id,
                account_id,
                current_balance,
                total_consumed,
                total_quota,
                recorded_at
            FROM balance_history
            WHERE account_id = ?1
            ORDER BY recorded_at DESC
            LIMIT 1
        "#;

        let row: Option<BalanceHistoryRow> = self
            .base
            .fetch_optional(
                sqlx::query_as(query).bind(account_id.as_str()),
                "Find latest balance history by account ID",
            )
            .await?;

        Ok(row.map(|r| r.into_record()))
    }

    async fn find_latest_by_account_id_on_date(
        &self,
        account_id: &AccountId,
        date: NaiveDate,
    ) -> Result<Option<BalanceHistoryRecord>, DomainError> {
        let query = r#"
            SELECT
                id,
                account_id,
                current_balance,
                total_consumed,
                total_quota,
                recorded_at
            FROM balance_history
            WHERE account_id = ?1 AND DATE(recorded_at) = ?2
            ORDER BY recorded_at DESC
            LIMIT 1
        "#;

        let row: Option<BalanceHistoryRow> = self
            .base
            .fetch_optional(
                sqlx::query_as(query)
                    .bind(account_id.as_str())
                    .bind(date.format("%Y-%m-%d").to_string()),
                "Find latest balance history by account ID and date",
            )
            .await?;

        Ok(row.map(|r| r.into_record()))
    }

    async fn list_all_daily_summaries(
        &self,
        account_id: &AccountId,
    ) -> Result<Vec<BalanceHistoryDailySummary>, DomainError> {
        let query = format!("{DAILY_SUMMARY_QUERY} ORDER BY days.day ASC");

        let rows: Vec<DailySummaryRow> = self
            .base
            .fetch_all(
                sqlx::query_as(&query).bind(account_id.as_str()),
                "List all daily summaries",
            )
            .await?;

        rows.into_iter().map(|r| r.try_into_summary()).collect()
    }

    async fn list_daily_summaries_in_range(
        &self,
        account_id: &AccountId,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<BalanceHistoryDailySummary>, DomainError> {
        let query = format!(
            "{DAILY_SUMMARY_QUERY} WHERE days.day >= ?2 AND days.day <= ?3 ORDER BY days.day ASC"
        );

        let rows: Vec<DailySummaryRow> = self
            .base
            .fetch_all(
                sqlx::query_as(&query)
                    .bind(account_id.as_str())
                    .bind(start_date.format("%Y-%m-%d").to_string())
                    .bind(end_date.format("%Y-%m-%d").to_string()),
                "List daily summaries in range",
            )
            .await?;

        rows.into_iter().map(|r| r.try_into_summary()).collect()
    }

    async fn find_daily_summary(
        &self,
        account_id: &AccountId,
        date: NaiveDate,
    ) -> Result<Option<BalanceHistoryDailySummary>, DomainError> {
        let query = format!("{DAILY_SUMMARY_QUERY} WHERE days.day = ?2 LIMIT 1");

        let row: Option<DailySummaryRow> = self
            .base
            .fetch_optional(
                sqlx::query_as(&query)
                    .bind(account_id.as_str())
                    .bind(date.format("%Y-%m-%d").to_string()),
                "Find daily summary",
            )
            .await?;

        row.map(|r| r.try_into_summary()).transpose()
    }

    async fn list_distinct_account_ids(&self) -> Result<Vec<AccountId>, DomainError> {
        let query =
            "SELECT account_id FROM balance_history UNION SELECT account_id FROM check_in_records";
        let ids: Vec<String> = sqlx::query_scalar(query)
            .fetch_all(self.base.pool())
            .await
            .map_err(|e| DomainError::Repository(format!("List distinct account ids: {e}")))?;

        Ok(ids
            .into_iter()
            .map(|id| AccountId::from_string(&id))
            .collect())
    }
}
