use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;

use neuradock_domain::shared::{AccountId, DomainError};
use neuradock_domain::token::{ApiToken, ModelLimits, TokenId, TokenRepository, TokenStatus};

use crate::persistence::unit_of_work::RepositoryErrorMapper;

#[derive(Debug, FromRow)]
struct TokenRow {
    id: i64,
    account_id: String,
    token_id: i64,
    token_name: String,
    token_key: String,
    status: i32,
    used_quota: i64,
    remain_quota: i64,
    unlimited_quota: i32,
    expired_time: Option<i64>,
    model_limits_allowed: Option<String>,
    model_limits_denied: Option<String>,
    fetched_at: String,
}

pub struct SqliteTokenRepository {
    pool: Arc<SqlitePool>,
}

impl SqliteTokenRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    fn row_to_domain(&self, row: TokenRow) -> Result<ApiToken, DomainError> {
        let status = TokenStatus::from_i32(row.status)
            .ok_or_else(|| DomainError::Validation(format!("Invalid token status: {}", row.status)))?;

        let expired_time = row.expired_time.and_then(|ts| {
            if ts == -1 {
                None
            } else {
                DateTime::from_timestamp(ts, 0)
            }
        });

        let model_limits = if let (Some(allowed_str), Some(denied_str)) = 
            (row.model_limits_allowed, row.model_limits_denied) {
            let allowed: Vec<String> = serde_json::from_str(&allowed_str)
                .unwrap_or_default();
            let denied: Vec<String> = serde_json::from_str(&denied_str)
                .unwrap_or_default();
            Some(ModelLimits { allowed, denied })
        } else {
            None
        };

        let fetched_at = DateTime::parse_from_rfc3339(&row.fetched_at)
            .map_err(|e| DomainError::Validation(format!("Invalid fetched_at: {}", e)))?
            .with_timezone(&Utc);

        Ok(ApiToken::new(
            TokenId::new(row.token_id),
            AccountId::from_string(&row.account_id),
            row.token_name,
            row.token_key,
            status,
            row.used_quota,
            row.remain_quota,
            row.unlimited_quota != 0,
            expired_time,
            model_limits,
        ))
    }
}

#[async_trait]
impl TokenRepository for SqliteTokenRepository {
    async fn save(&self, token: &ApiToken) -> Result<(), DomainError> {
        let model_limits_allowed = token.model_limits()
            .map(|limits| serde_json::to_string(&limits.allowed).unwrap_or_default());
        let model_limits_denied = token.model_limits()
            .map(|limits| serde_json::to_string(&limits.denied).unwrap_or_default());

        let expired_time = token.expired_time().map(|dt| dt.timestamp());

        sqlx::query(
            r#"
            INSERT INTO api_tokens (
                account_id, token_id, token_name, token_key, status,
                used_quota, remain_quota, unlimited_quota, expired_time,
                model_limits_allowed, model_limits_denied, fetched_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(account_id, token_id) DO UPDATE SET
                token_name = excluded.token_name,
                token_key = excluded.token_key,
                status = excluded.status,
                used_quota = excluded.used_quota,
                remain_quota = excluded.remain_quota,
                unlimited_quota = excluded.unlimited_quota,
                expired_time = excluded.expired_time,
                model_limits_allowed = excluded.model_limits_allowed,
                model_limits_denied = excluded.model_limits_denied,
                fetched_at = excluded.fetched_at
            "#,
        )
        .bind(token.account_id().to_string())
        .bind(token.id().value())
        .bind(token.name())
        .bind(token.key())
        .bind(token.status().to_i32())
        .bind(token.used_quota())
        .bind(token.remain_quota())
        .bind(if token.unlimited_quota() { 1 } else { 0 })
        .bind(expired_time)
        .bind(model_limits_allowed)
        .bind(model_limits_denied)
        .bind(token.fetched_at().to_rfc3339())
        .execute(&*self.pool)
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Save token"))?;

        Ok(())
    }

    async fn save_batch(&self, tokens: Vec<ApiToken>) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Begin transaction"))?;

        for token in tokens {
            let model_limits_allowed = token.model_limits()
                .map(|limits| serde_json::to_string(&limits.allowed).unwrap_or_default());
            let model_limits_denied = token.model_limits()
                .map(|limits| serde_json::to_string(&limits.denied).unwrap_or_default());

            let expired_time = token.expired_time().map(|dt| dt.timestamp());

            sqlx::query(
                r#"
                INSERT INTO api_tokens (
                    account_id, token_id, token_name, token_key, status,
                    used_quota, remain_quota, unlimited_quota, expired_time,
                    model_limits_allowed, model_limits_denied, fetched_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(account_id, token_id) DO UPDATE SET
                    token_name = excluded.token_name,
                    token_key = excluded.token_key,
                    status = excluded.status,
                    used_quota = excluded.used_quota,
                    remain_quota = excluded.remain_quota,
                    unlimited_quota = excluded.unlimited_quota,
                    expired_time = excluded.expired_time,
                    model_limits_allowed = excluded.model_limits_allowed,
                    model_limits_denied = excluded.model_limits_denied,
                    fetched_at = excluded.fetched_at
                "#,
            )
            .bind(token.account_id().to_string())
            .bind(token.id().value())
            .bind(token.name())
            .bind(token.key())
            .bind(token.status().to_i32())
            .bind(token.used_quota())
            .bind(token.remain_quota())
            .bind(if token.unlimited_quota() { 1 } else { 0 })
            .bind(expired_time)
            .bind(model_limits_allowed)
            .bind(model_limits_denied)
            .bind(token.fetched_at().to_rfc3339())
            .execute(&mut *tx)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Save token in batch"))?;
        }

        tx.commit().await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Commit transaction"))?;

        Ok(())
    }

    async fn find_by_id(&self, id: &TokenId) -> Result<Option<ApiToken>, DomainError> {
        let row = sqlx::query_as::<_, TokenRow>(
            r#"
            SELECT id, account_id, token_id, token_name, token_key, status,
                   used_quota, remain_quota, unlimited_quota, expired_time,
                   model_limits_allowed, model_limits_denied, fetched_at
            FROM api_tokens
            WHERE token_id = ?
            "#,
        )
        .bind(id.value())
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find token by id"))?;

        match row {
            Some(row) => Ok(Some(self.row_to_domain(row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_account(&self, account_id: &AccountId) -> Result<Vec<ApiToken>, DomainError> {
        let rows = sqlx::query_as::<_, TokenRow>(
            r#"
            SELECT id, account_id, token_id, token_name, token_key, status,
                   used_quota, remain_quota, unlimited_quota, expired_time,
                   model_limits_allowed, model_limits_denied, fetched_at
            FROM api_tokens
            WHERE account_id = ?
            ORDER BY token_id
            "#,
        )
        .bind(account_id.to_string())
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find tokens by account"))?;

        let tokens: Result<Vec<ApiToken>, DomainError> = rows
            .into_iter()
            .map(|row| self.row_to_domain(row))
            .collect();

        tokens
    }

    async fn delete_by_account(&self, account_id: &AccountId) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM api_tokens WHERE account_id = ?")
            .bind(account_id.to_string())
            .execute(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Delete tokens by account"))?;

        Ok(())
    }
}
