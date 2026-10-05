use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, SqliteConnection, SqlitePool};
use std::sync::Arc;

use crate::persistence::SqliteRepositoryBase;
use neuradock_domain::balance_history::{
    AttemptCompletion, BalanceHistoryDailySummary, BalanceHistoryRecord, BalanceHistoryRepository,
    CheckInAttempt, CheckInOutcome, CheckInRecord, CheckInSource, CheckInTrigger,
};
use neuradock_domain::shared::{AccountId, DomainError, ProviderId};

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
struct EvidenceRow {
    account_id: String,
    quota_day: String,
    reward_cents: Option<i64>,
    source: String,
    recorded_at: DateTime<Utc>,
    attempt_id: Option<i64>,
}

impl EvidenceRow {
    fn try_into_record(self) -> Result<CheckInRecord, DomainError> {
        Ok(CheckInRecord::restore(
            AccountId::from_string(&self.account_id),
            parse_day(&self.quota_day)?,
            self.reward_cents,
            CheckInSource::parse(&self.source)?,
            self.recorded_at,
            self.attempt_id,
        ))
    }
}

#[derive(FromRow)]
struct AttemptRow {
    id: i64,
    account_id: String,
    provider_id: String,
    quota_day: String,
    trigger_kind: String,
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    outcome: Option<String>,
    error_code: Option<String>,
    http_status: Option<i64>,
    duration_ms: Option<i64>,
    reward_cents: Option<i64>,
    source: Option<String>,
}

impl AttemptRow {
    fn try_into_attempt(self) -> Result<CheckInAttempt, DomainError> {
        Ok(CheckInAttempt::restore(
            self.id,
            AccountId::from_string(&self.account_id),
            ProviderId::from_string(&self.provider_id),
            parse_day(&self.quota_day)?,
            CheckInTrigger::parse(&self.trigger_kind)?,
            self.started_at,
            self.finished_at,
            self.outcome
                .as_deref()
                .map(CheckInOutcome::parse)
                .transpose()?,
            self.error_code,
            self.http_status,
            self.duration_ms,
            self.reward_cents,
            self.source
                .as_deref()
                .map(CheckInSource::parse)
                .transpose()?,
        ))
    }
}

#[derive(FromRow)]
struct DailySummaryRow {
    quota_day: String,
    daily_total_quota: f64,
    daily_balance: f64,
    daily_consumed: f64,
    check_in_reward: Option<f64>,
    check_in_source: Option<String>,
}

impl DailySummaryRow {
    fn try_into_summary(self) -> Result<BalanceHistoryDailySummary, DomainError> {
        let source = self
            .check_in_source
            .as_deref()
            .map(CheckInSource::parse)
            .transpose()?;

        Ok(BalanceHistoryDailySummary::restore(
            parse_day(&self.quota_day)?,
            self.daily_total_quota,
            self.daily_balance,
            self.daily_consumed,
        )
        .with_check_in(self.check_in_reward, source))
    }
}

fn parse_day(value: &str) -> Result<NaiveDate, DomainError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|e| DomainError::Validation(format!("Invalid quota day {value:?}: {e}")))
}

/// Snapshot days and evidence days must share one axis, so a snapshot is
/// bucketed by its provider's quota day too -- not by its UTC calendar day.
const DAILY_SUMMARY_QUERY: &str = r#"
    WITH cfg AS (
        SELECT COALESCE(MAX(p.check_in_day_offset_minutes), 480) AS offset_minutes
        FROM accounts a
        LEFT JOIN providers p ON p.id = a.provider_id
        WHERE a.id = ?1
    ),
    days AS (
        SELECT DATE(recorded_at, printf('%+d minutes', (SELECT offset_minutes FROM cfg))) AS day
        FROM balance_history WHERE account_id = ?1
        UNION
        SELECT quota_day AS day FROM check_in_records WHERE account_id = ?1
    )
    SELECT days.day AS quota_day,
           COALESCE(b.total_quota, 0.0) AS daily_total_quota,
           COALESCE(b.current_balance, 0.0) AS daily_balance,
           COALESCE(b.total_consumed, 0.0) AS daily_consumed,
           c.reward_cents / 100.0 AS check_in_reward,
           c.source AS check_in_source
    FROM days
    LEFT JOIN balance_history b ON b.id = (
        SELECT id FROM balance_history
        WHERE account_id = ?1
          AND DATE(recorded_at, printf('%+d minutes', (SELECT offset_minutes FROM cfg))) <= days.day
        ORDER BY recorded_at DESC, id DESC
        LIMIT 1
    )
    LEFT JOIN check_in_records c ON c.account_id = ?1 AND c.quota_day = days.day
"#;

const EVIDENCE_COLUMNS: &str =
    "account_id, quota_day, reward_cents, source, recorded_at, attempt_id";

/// Ranks an existing evidence row by source. Mirrors `CheckInSource::precedence`
/// so the ledger, the domain and the rebuild migration agree on who wins a day.
const EXISTING_EVIDENCE_PRECEDENCE: &str = r#"
    CASE check_in_records.source
        WHEN 'manual' THEN 0
        WHEN 'provider_response' THEN 1
        WHEN 'provider_rule' THEN 2
        WHEN 'confirmed_unknown' THEN 3
        WHEN 'balance_delta' THEN 4
        ELSE 5
    END
"#;

const ATTEMPT_COLUMNS: &str = "id, account_id, provider_id, quota_day, trigger_kind, started_at, \
     finished_at, outcome, error_code, http_status, duration_ms, reward_cents, source";

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

/// Upsert one evidence row, applying the precedence ladder.
async fn upsert_evidence(
    conn: &mut SqliteConnection,
    record: &CheckInRecord,
) -> Result<bool, DomainError> {
    let query = format!(
        r#"
        INSERT INTO check_in_records ({EVIDENCE_COLUMNS}, confidence)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(account_id, quota_day) DO UPDATE SET
            reward_cents = excluded.reward_cents,
            source       = excluded.source,
            confidence   = excluded.confidence,
            recorded_at  = excluded.recorded_at,
            attempt_id   = COALESCE(excluded.attempt_id, check_in_records.attempt_id)
        WHERE ?8 < {EXISTING_EVIDENCE_PRECEDENCE}
           OR (check_in_records.reward_cents IS NULL AND excluded.reward_cents IS NOT NULL)
        "#
    );

    let result = sqlx::query(&query)
        .bind(record.account_id().as_str())
        .bind(record.quota_day().format("%Y-%m-%d").to_string())
        .bind(record.reward_cents())
        .bind(record.source().as_str())
        .bind(record.recorded_at())
        .bind(record.attempt_id())
        .bind(record.confidence().as_str())
        .bind(i64::from(record.source().precedence()))
        .execute(&mut *conn)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Save check-in evidence: {e}")))?;

    Ok(result.rows_affected() > 0)
}

/// Recompute the account's cached check-in timestamps from the ledger. The
/// ledger is the source of truth, so this can always be re-run to heal drift.
async fn refresh_cache(conn: &mut SqliteConnection, account_id: &str) -> Result<(), DomainError> {
    sqlx::query(
        r#"
        UPDATE accounts SET
            last_confirmed_at = (
                SELECT MAX(recorded_at) FROM check_in_records
                WHERE account_id = ?1 AND confidence IN ('confirmed', 'manual')
            ),
            last_evidence_at = (
                SELECT MAX(recorded_at) FROM check_in_records WHERE account_id = ?1
            )
        WHERE id = ?1
        "#,
    )
    .bind(account_id)
    .execute(&mut *conn)
    .await
    .map_err(|e| DomainError::Infrastructure(format!("Refresh check-in cache: {e}")))?;

    Ok(())
}

async fn insert_snapshot(
    conn: &mut SqliteConnection,
    record: &BalanceHistoryRecord,
) -> Result<(), DomainError> {
    sqlx::query(
        r#"
        INSERT OR REPLACE INTO balance_history (
            id, account_id, current_balance, total_consumed, total_quota, recorded_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
    )
    .bind(record.id())
    .bind(record.account_id().as_str())
    .bind(record.current_balance())
    .bind(record.total_consumed())
    .bind(record.total_quota())
    .bind(record.recorded_at())
    .execute(&mut *conn)
    .await
    .map_err(|e| DomainError::Infrastructure(format!("Save balance history: {e}")))?;

    Ok(())
}

fn transaction_error(operation: &str, error: sqlx::Error) -> DomainError {
    DomainError::Infrastructure(format!("{operation}: {error}"))
}

#[async_trait]
impl BalanceHistoryRepository for SqliteBalanceHistoryRepository {
    async fn save_check_in(&self, record: &CheckInRecord) -> Result<bool, DomainError> {
        let mut tx = self
            .base
            .pool()
            .begin()
            .await
            .map_err(|e| transaction_error("Begin check-in evidence write", e))?;

        let changed = upsert_evidence(&mut tx, record).await?;
        refresh_cache(&mut tx, record.account_id().as_str()).await?;

        tx.commit()
            .await
            .map_err(|e| transaction_error("Commit check-in evidence write", e))?;

        Ok(changed)
    }

    async fn save(&self, record: &BalanceHistoryRecord) -> Result<(), DomainError> {
        let mut conn = self
            .base
            .pool()
            .acquire()
            .await
            .map_err(|e| transaction_error("Acquire connection", e))?;

        insert_snapshot(&mut conn, record).await
    }

    async fn save_balance_observation(
        &self,
        snapshot: &BalanceHistoryRecord,
        inferred: Option<&CheckInRecord>,
    ) -> Result<bool, DomainError> {
        let mut tx = self
            .base
            .pool()
            .begin()
            .await
            .map_err(|e| transaction_error("Begin balance observation", e))?;

        insert_snapshot(&mut tx, snapshot).await?;

        let mut changed = false;
        if let Some(record) = inferred {
            changed = upsert_evidence(&mut tx, record).await?;
        }
        refresh_cache(&mut tx, snapshot.account_id().as_str()).await?;

        tx.commit()
            .await
            .map_err(|e| transaction_error("Commit balance observation", e))?;

        Ok(changed)
    }

    async fn find_evidence(
        &self,
        account_id: &AccountId,
        quota_day: NaiveDate,
    ) -> Result<Option<CheckInRecord>, DomainError> {
        let query = format!(
            "SELECT {EVIDENCE_COLUMNS} FROM check_in_records WHERE account_id = ?1 AND quota_day = ?2 LIMIT 1"
        );

        let row: Option<EvidenceRow> = self
            .base
            .fetch_optional(
                sqlx::query_as(&query)
                    .bind(account_id.as_str())
                    .bind(quota_day.format("%Y-%m-%d").to_string()),
                "Find check-in evidence",
            )
            .await?;

        row.map(EvidenceRow::try_into_record).transpose()
    }

    async fn list_evidence_in_range(
        &self,
        account_id: &AccountId,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<CheckInRecord>, DomainError> {
        let query = format!(
            "SELECT {EVIDENCE_COLUMNS} FROM check_in_records \
             WHERE account_id = ?1 AND quota_day >= ?2 AND quota_day <= ?3 ORDER BY quota_day ASC"
        );

        let rows: Vec<EvidenceRow> = self
            .base
            .fetch_all(
                sqlx::query_as(&query)
                    .bind(account_id.as_str())
                    .bind(start_date.format("%Y-%m-%d").to_string())
                    .bind(end_date.format("%Y-%m-%d").to_string()),
                "List check-in evidence in range",
            )
            .await?;

        rows.into_iter().map(EvidenceRow::try_into_record).collect()
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

    async fn start_attempt(&self, attempt: &CheckInAttempt) -> Result<i64, DomainError> {
        let result = sqlx::query(
            "INSERT INTO check_in_attempts \
             (account_id, provider_id, quota_day, trigger_kind, started_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(attempt.account_id().as_str())
        .bind(attempt.provider_id().as_str())
        .bind(attempt.quota_day().format("%Y-%m-%d").to_string())
        .bind(attempt.trigger().as_str())
        .bind(attempt.started_at())
        .execute(self.base.pool())
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Start check-in attempt: {e}")))?;

        Ok(result.last_insert_rowid())
    }

    async fn finish_attempt(
        &self,
        attempt_id: i64,
        completion: &AttemptCompletion,
    ) -> Result<(), DomainError> {
        let mut tx = self
            .base
            .pool()
            .begin()
            .await
            .map_err(|e| transaction_error("Begin attempt completion", e))?;

        // Read-modify-write so the duration and the outcome transitions stay in
        // the domain instead of being re-derived in SQL.
        let query = format!("SELECT {ATTEMPT_COLUMNS} FROM check_in_attempts WHERE id = ?1");
        let row: Option<AttemptRow> = sqlx::query_as(&query)
            .bind(attempt_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| transaction_error("Load check-in attempt", e))?;

        let Some(row) = row else {
            return Err(DomainError::NotFound(format!(
                "Check-in attempt {attempt_id}"
            )));
        };

        let mut attempt = row.try_into_attempt()?;
        attempt.finish(completion.clone());

        sqlx::query(
            "UPDATE check_in_attempts SET \
                finished_at = ?2, outcome = ?3, error_code = ?4, http_status = ?5, \
                duration_ms = ?6, reward_cents = ?7, source = ?8 \
             WHERE id = ?1",
        )
        .bind(attempt_id)
        .bind(attempt.finished_at())
        .bind(attempt.outcome().map(|outcome| outcome.as_str()))
        .bind(attempt.error_code())
        .bind(attempt.http_status())
        .bind(attempt.duration_ms())
        .bind(attempt.reward_cents())
        .bind(attempt.source().map(|source| source.as_str()))
        .execute(&mut *tx)
        .await
        .map_err(|e| transaction_error("Finish check-in attempt", e))?;

        tx.commit()
            .await
            .map_err(|e| transaction_error("Commit attempt completion", e))?;

        Ok(())
    }

    async fn find_open_attempt(
        &self,
        account_id: &AccountId,
        quota_day: NaiveDate,
        started_after: DateTime<Utc>,
    ) -> Result<Option<CheckInAttempt>, DomainError> {
        let query = format!(
            "SELECT {ATTEMPT_COLUMNS} FROM check_in_attempts \
             WHERE account_id = ?1 AND quota_day = ?2 \
               AND finished_at IS NULL AND outcome IS NULL AND started_at >= ?3 \
             ORDER BY id DESC LIMIT 1"
        );

        let row: Option<AttemptRow> = self
            .base
            .fetch_optional(
                sqlx::query_as(&query)
                    .bind(account_id.as_str())
                    .bind(quota_day.format("%Y-%m-%d").to_string())
                    .bind(started_after),
                "Find open check-in attempt",
            )
            .await?;

        row.map(AttemptRow::try_into_attempt).transpose()
    }

    async fn list_attempts_on_day(
        &self,
        account_id: &AccountId,
        quota_day: NaiveDate,
    ) -> Result<Vec<CheckInAttempt>, DomainError> {
        let query = format!(
            "SELECT {ATTEMPT_COLUMNS} FROM check_in_attempts \
             WHERE account_id = ?1 AND quota_day = ?2 ORDER BY started_at ASC, id ASC"
        );

        let rows: Vec<AttemptRow> = self
            .base
            .fetch_all(
                sqlx::query_as(&query)
                    .bind(account_id.as_str())
                    .bind(quota_day.format("%Y-%m-%d").to_string()),
                "List check-in attempts on a day",
            )
            .await?;

        rows.into_iter().map(AttemptRow::try_into_attempt).collect()
    }

    async fn refresh_check_in_cache(&self, account_id: &AccountId) -> Result<(), DomainError> {
        let mut conn = self
            .base
            .pool()
            .acquire()
            .await
            .map_err(|e| transaction_error("Acquire connection", e))?;

        refresh_cache(&mut conn, account_id.as_str()).await
    }

    async fn list_attempts(
        &self,
        account_id: Option<&AccountId>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CheckInAttempt>, DomainError> {
        let query = format!(
            "SELECT {ATTEMPT_COLUMNS} FROM check_in_attempts \
             WHERE (?1 IS NULL OR account_id = ?1) \
             ORDER BY started_at DESC, id DESC LIMIT ?2 OFFSET ?3"
        );

        let rows: Vec<AttemptRow> = self
            .base
            .fetch_all(
                sqlx::query_as(&query)
                    .bind(account_id.map(|id| id.as_str()))
                    .bind(limit)
                    .bind(offset),
                "List check-in attempts",
            )
            .await?;

        rows.into_iter().map(AttemptRow::try_into_attempt).collect()
    }

    async fn count_attempts(&self, account_id: Option<&AccountId>) -> Result<i64, DomainError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM check_in_attempts WHERE (?1 IS NULL OR account_id = ?1)",
        )
        .bind(account_id.map(|id| id.as_str()))
        .fetch_one(self.base.pool())
        .await
        .map_err(|e| DomainError::Repository(format!("Count check-in attempts: {e}")))?;

        Ok(count)
    }

    async fn list_attempts_in_range(
        &self,
        account_id: &AccountId,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<CheckInAttempt>, DomainError> {
        let query = format!(
            "SELECT {ATTEMPT_COLUMNS} FROM check_in_attempts \
             WHERE account_id = ?1 AND quota_day >= ?2 AND quota_day <= ?3 \
             ORDER BY started_at ASC, id ASC"
        );

        let rows: Vec<AttemptRow> = self
            .base
            .fetch_all(
                sqlx::query_as(&query)
                    .bind(account_id.as_str())
                    .bind(start_date.format("%Y-%m-%d").to_string())
                    .bind(end_date.format("%Y-%m-%d").to_string()),
                "List check-in attempts in range",
            )
            .await?;

        rows.into_iter().map(AttemptRow::try_into_attempt).collect()
    }
}
