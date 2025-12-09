use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;

use crate::persistence::ResultExt;
use neuradock_domain::balance::{Balance, BalanceRepository};
use neuradock_domain::shared::{AccountId, DomainError};

#[derive(FromRow)]
struct BalanceRow {
    account_id: String,
    current: f64,
    total_consumed: f64,
    total_income: f64,
    last_checked_at: DateTime<Utc>,
}

impl BalanceRow {
    fn to_balance(self) -> Balance {
        Balance::restore(
            AccountId::from_string(&self.account_id),
            self.current,
            self.total_consumed,
            self.total_income,
            self.last_checked_at,
        )
    }
}

use crate::persistence::SqliteRepositoryBase;

pub struct SqliteBalanceRepository {
    base: SqliteRepositoryBase,
}

impl SqliteBalanceRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self {
            base: SqliteRepositoryBase::new(pool),
        }
    }
}

#[async_trait]
impl BalanceRepository for SqliteBalanceRepository {
    async fn save(&self, balance: &Balance) -> Result<(), DomainError> {
        let query = r#"
            INSERT INTO balances (account_id, current, total_consumed, total_income, last_checked_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(account_id) DO UPDATE SET
                current = ?2,
                total_consumed = ?3,
                total_income = ?4,
                last_checked_at = ?5
        "#;

        sqlx::query(query)
            .bind(balance.account_id().as_str())
            .bind(balance.current())
            .bind(balance.total_consumed())
            .bind(balance.total_income())
            .bind(balance.last_checked_at())
            .execute(self.base.pool())
            .await
            .map_repo_error("Save balance")?;

        Ok(())
    }

    async fn find_by_account_id(
        &self,
        account_id: &AccountId,
    ) -> Result<Option<Balance>, DomainError> {
        let query = "SELECT account_id, current, total_consumed, total_income, last_checked_at FROM balances WHERE account_id = ?1";

        let row: Option<BalanceRow> = self
            .base
            .fetch_optional(
                sqlx::query_as(query).bind(account_id.as_str()),
                "Find balance by account ID",
            )
            .await?;

        Ok(row.map(|r| r.to_balance()))
    }

    async fn delete(&self, account_id: &AccountId) -> Result<(), DomainError> {
        let query = "DELETE FROM balances WHERE account_id = ?1";

        self.base
            .execute(
                sqlx::query(query).bind(account_id.as_str()),
                "Delete balance",
            )
            .await?;

        Ok(())
    }

    async fn find_all(&self) -> Result<Vec<Balance>, DomainError> {
        let query = "SELECT account_id, current, total_consumed, total_income, last_checked_at FROM balances ORDER BY last_checked_at DESC";

        let rows: Vec<BalanceRow> = self
            .base
            .fetch_all(sqlx::query_as(query), "Find all balances")
            .await?;

        Ok(rows.into_iter().map(|r| r.to_balance()).collect())
    }

    async fn find_stale_balances(&self, hours_threshold: i64) -> Result<Vec<Balance>, DomainError> {
        let threshold = Utc::now() - chrono::Duration::hours(hours_threshold);
        let query = "SELECT account_id, current, total_consumed, total_income, last_checked_at FROM balances WHERE last_checked_at < ?1 ORDER BY last_checked_at ASC";

        let rows: Vec<BalanceRow> = self
            .base
            .fetch_all(sqlx::query_as(query).bind(threshold), "Find stale balances")
            .await?;

        Ok(rows.into_iter().map(|r| r.to_balance()).collect())
    }
}
