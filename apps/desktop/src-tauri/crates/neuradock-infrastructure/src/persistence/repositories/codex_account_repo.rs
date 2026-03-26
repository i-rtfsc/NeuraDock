use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;

use crate::persistence::RepositoryErrorMapper;
use neuradock_domain::codex::{
    CodexAccount, CodexAccountId, CodexAccountRepository, CodexAccountSource, CodexAccountStatus,
    CodexRateLimitWindow,
};
use neuradock_domain::shared::DomainError;

#[derive(FromRow)]
struct CodexAccountRow {
    id: String,
    email: String,
    password: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    id_token: Option<String>,
    account_id: Option<String>,
    web_session_cookie: Option<String>,
    web_session_device_id: Option<String>,
    plan_type: Option<String>,
    has_credits: Option<i64>,
    is_unlimited: Option<i64>,
    credit_balance: Option<String>,
    primary_used_percent: Option<f64>,
    primary_window_minutes: Option<i64>,
    primary_resets_at: Option<String>,
    secondary_used_percent: Option<f64>,
    secondary_window_minutes: Option<i64>,
    secondary_resets_at: Option<String>,
    quota_checked_at: Option<String>,
    token_expires_at: Option<String>,
    last_refresh_at: Option<String>,
    tempmail_token: Option<String>,
    source: String,
    status: String,
    created_at: String,
    updated_at: String,
}

fn parse_opt_dt(s: Option<String>) -> Option<DateTime<Utc>> {
    s.and_then(|v| {
        DateTime::parse_from_rfc3339(&v)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    })
}

fn build_rate_limit_window(
    used_percent: Option<f64>,
    window_minutes: Option<i64>,
    resets_at: Option<String>,
) -> Option<CodexRateLimitWindow> {
    if used_percent.is_none() && window_minutes.is_none() && resets_at.is_none() {
        return None;
    }

    Some(CodexRateLimitWindow::new(
        used_percent.unwrap_or(0.0),
        window_minutes,
        parse_opt_dt(resets_at),
    ))
}

impl CodexAccountRow {
    fn into_domain(self) -> Result<CodexAccount, DomainError> {
        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map_err(|e| DomainError::DataIntegrity(format!("Invalid created_at: {}", e)))?
            .with_timezone(&Utc);
        let updated_at = DateTime::parse_from_rfc3339(&self.updated_at)
            .map_err(|e| DomainError::DataIntegrity(format!("Invalid updated_at: {}", e)))?
            .with_timezone(&Utc);

        Ok(CodexAccount::restore(
            CodexAccountId::from_string(&self.id),
            self.email,
            self.password,
            self.access_token,
            self.refresh_token,
            self.id_token,
            self.account_id,
            self.web_session_cookie,
            self.web_session_device_id,
            self.plan_type,
            self.has_credits.map(|v| v != 0),
            self.is_unlimited.map(|v| v != 0),
            self.credit_balance,
            build_rate_limit_window(
                self.primary_used_percent,
                self.primary_window_minutes,
                self.primary_resets_at,
            ),
            build_rate_limit_window(
                self.secondary_used_percent,
                self.secondary_window_minutes,
                self.secondary_resets_at,
            ),
            parse_opt_dt(self.quota_checked_at),
            parse_opt_dt(self.token_expires_at),
            parse_opt_dt(self.last_refresh_at),
            self.tempmail_token,
            CodexAccountSource::from_str(&self.source),
            CodexAccountStatus::from_str(&self.status),
            created_at,
            updated_at,
        ))
    }
}

pub struct SqliteCodexAccountRepository {
    pool: Arc<SqlitePool>,
}

impl SqliteCodexAccountRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CodexAccountRepository for SqliteCodexAccountRepository {
    async fn save(&self, account: &CodexAccount) -> Result<(), DomainError> {
        let query = r#"
            INSERT INTO codex_accounts (
                id, email, password,
                access_token, refresh_token, id_token, account_id,
                web_session_cookie, web_session_device_id,
                plan_type, has_credits, is_unlimited, credit_balance,
                primary_used_percent, primary_window_minutes, primary_resets_at,
                secondary_used_percent, secondary_window_minutes, secondary_resets_at,
                quota_checked_at,
                token_expires_at, last_refresh_at, tempmail_token,
                source, status, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23, ?24, ?25, ?26, ?27
            )
            ON CONFLICT(id) DO UPDATE SET
                email            = ?2,
                password         = ?3,
                access_token     = ?4,
                refresh_token    = ?5,
                id_token         = ?6,
                account_id       = ?7,
                web_session_cookie   = ?8,
                web_session_device_id = ?9,
                plan_type        = ?10,
                has_credits      = ?11,
                is_unlimited     = ?12,
                credit_balance   = ?13,
                primary_used_percent   = ?14,
                primary_window_minutes = ?15,
                primary_resets_at      = ?16,
                secondary_used_percent   = ?17,
                secondary_window_minutes = ?18,
                secondary_resets_at      = ?19,
                quota_checked_at = ?20,
                token_expires_at = ?21,
                last_refresh_at  = ?22,
                tempmail_token   = ?23,
                source           = ?24,
                status           = ?25,
                updated_at       = ?27
        "#;

        sqlx::query(query)
            .bind(account.id().as_str())
            .bind(account.email())
            .bind(account.password())
            .bind(account.access_token())
            .bind(account.refresh_token())
            .bind(account.id_token())
            .bind(account.account_id())
            .bind(account.web_session_cookie())
            .bind(account.web_session_device_id())
            .bind(account.plan_type())
            .bind(account.has_credits().map(|v| v as i64))
            .bind(account.is_unlimited().map(|v| v as i64))
            .bind(account.credit_balance())
            .bind(account.primary_window().map(|window| window.used_percent()))
            .bind(account.primary_window().and_then(|window| window.window_minutes()))
            .bind(account.primary_window().and_then(|window| window.resets_at()).map(|dt| dt.to_rfc3339()))
            .bind(account.secondary_window().map(|window| window.used_percent()))
            .bind(account.secondary_window().and_then(|window| window.window_minutes()))
            .bind(account.secondary_window().and_then(|window| window.resets_at()).map(|dt| dt.to_rfc3339()))
            .bind(account.quota_checked_at().map(|dt| dt.to_rfc3339()))
            .bind(account.token_expires_at().map(|dt| dt.to_rfc3339()))
            .bind(account.last_refresh_at().map(|dt| dt.to_rfc3339()))
            .bind(account.tempmail_token())
            .bind(account.source().as_str())
            .bind(account.status().as_str())
            .bind(account.created_at().to_rfc3339())
            .bind(account.updated_at().to_rfc3339())
            .execute(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Save codex account"))?;

        Ok(())
    }

    async fn find_by_id(&self, id: &CodexAccountId) -> Result<Option<CodexAccount>, DomainError> {
        let row: Option<CodexAccountRow> = sqlx::query_as(
            "SELECT * FROM codex_accounts WHERE id = ?1",
        )
        .bind(id.as_str())
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find codex account by id"))?;

        row.map(|r| r.into_domain()).transpose()
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<CodexAccount>, DomainError> {
        let row: Option<CodexAccountRow> = sqlx::query_as(
            "SELECT * FROM codex_accounts WHERE email = ?1",
        )
        .bind(email.to_lowercase())
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find codex account by email"))?;

        row.map(|r| r.into_domain()).transpose()
    }

    async fn find_all(&self) -> Result<Vec<CodexAccount>, DomainError> {
        let rows: Vec<CodexAccountRow> = sqlx::query_as(
            "SELECT * FROM codex_accounts ORDER BY created_at DESC",
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find all codex accounts"))?;

        rows.into_iter().map(|r| r.into_domain()).collect()
    }

    async fn delete(&self, id: &CodexAccountId) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM codex_accounts WHERE id = ?1")
            .bind(id.as_str())
            .execute(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Delete codex account"))?;
        Ok(())
    }

    async fn count(&self) -> Result<i64, DomainError> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM codex_accounts")
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Count codex accounts"))?;
        Ok(count)
    }
}
