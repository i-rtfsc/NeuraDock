use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;

use crate::persistence::SqliteRepositoryBase;
use neuradock_domain::balance_history::{BalanceHistoryRecord, BalanceHistoryRepository};
use neuradock_domain::shared::{AccountId, DomainError};

#[derive(FromRow)]
struct BalanceHistoryRow {
    id: String,
    account_id: String,
    current_balance: f64,
    total_consumed: f64,
    total_income: f64,
    recorded_at: DateTime<Utc>,
}

impl BalanceHistoryRow {
    fn to_record(self) -> BalanceHistoryRecord {
        BalanceHistoryRecord::restore(
            self.id,
            AccountId::from_string(&self.account_id),
            self.current_balance,
            self.total_consumed,
            self.total_income,
            self.recorded_at,
        )
    }
}

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
    async fn save(&self, record: &BalanceHistoryRecord) -> Result<(), DomainError> {
        let query = r#"
            INSERT OR REPLACE INTO balance_history (
                id,
                account_id,
                current_balance,
                total_consumed,
                total_income,
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
                    .bind(record.total_income())
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
                total_income,
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

        Ok(row.map(|r| r.to_record()))
    }
}

