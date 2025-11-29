use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json;
use sqlx::FromRow;
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::domain::account::{Account, AccountRepository, Credentials};
use crate::domain::shared::{AccountId, DomainError, ProviderId};
use crate::infrastructure::persistence::RepositoryErrorMapper;

#[derive(FromRow)]
struct AccountRow {
    id: String,
    name: String,
    provider_id: String,
    cookies: String,
    api_user: String,
    enabled: bool,
    last_check_in: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    auto_checkin_enabled: bool,
    auto_checkin_hour: i64,
    auto_checkin_minute: i64,
    last_login_at: Option<DateTime<Utc>>,
    session_token: Option<String>,
    session_expires_at: Option<DateTime<Utc>>,
    last_balance_check_at: Option<DateTime<Utc>>,
    current_balance: Option<f64>,
    total_consumed: Option<f64>,
    total_income: Option<f64>,
}

impl AccountRow {
    fn to_account(self) -> Result<Account, DomainError> {
        let cookies = serde_json::from_str(&self.cookies)
            .map_err(|e| RepositoryErrorMapper::map_json_error(e, "Deserialize account cookies"))?;
        let credentials = Credentials::new(cookies, self.api_user);

        Ok(Account::restore(
            AccountId::from_string(&self.id),
            self.name,
            ProviderId::from_string(&self.provider_id),
            credentials,
            self.enabled,
            self.last_check_in,
            self.created_at,
            self.auto_checkin_enabled,
            self.auto_checkin_hour as u8,
            self.auto_checkin_minute as u8,
            self.last_login_at,
            self.session_token,
            self.session_expires_at,
            self.last_balance_check_at,
            self.current_balance,
            self.total_consumed,
            self.total_income,
        ))
    }
}

pub struct SqliteAccountRepository {
    pool: Arc<SqlitePool>,
}

impl SqliteAccountRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AccountRepository for SqliteAccountRepository {
    async fn save(&self, account: &Account) -> Result<(), DomainError> {
        let query = r#"
            INSERT INTO accounts (id, name, provider_id, cookies, api_user, enabled, last_check_in, created_at, auto_checkin_enabled, auto_checkin_hour, auto_checkin_minute, last_login_at, session_token, session_expires_at, last_balance_check_at, current_balance, total_consumed, total_income)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
            ON CONFLICT(id) DO UPDATE SET
                name = ?2,
                provider_id = ?3,
                cookies = ?4,
                api_user = ?5,
                enabled = ?6,
                last_check_in = ?7,
                auto_checkin_enabled = ?9,
                auto_checkin_hour = ?10,
                auto_checkin_minute = ?11,
                last_login_at = ?12,
                session_token = ?13,
                session_expires_at = ?14,
                last_balance_check_at = ?15,
                current_balance = ?16,
                total_consumed = ?17,
                total_income = ?18
        "#;

        let cookies_json = serde_json::to_string(account.credentials().cookies())
            .map_err(|e| RepositoryErrorMapper::map_json_error(e, "Serialize account cookies"))?;

        sqlx::query(query)
            .bind(account.id().as_str())
            .bind(account.name())
            .bind(account.provider_id().as_str())
            .bind(cookies_json)
            .bind(account.credentials().api_user())
            .bind(account.is_enabled())
            .bind(account.last_check_in())
            .bind(account.created_at())
            .bind(account.auto_checkin_enabled())
            .bind(account.auto_checkin_hour() as i64)
            .bind(account.auto_checkin_minute() as i64)
            .bind(account.last_login_at())
            .bind(account.session_token())
            .bind(account.session_expires_at())
            .bind(account.last_balance_check_at())
            .bind(account.current_balance())
            .bind(account.total_consumed())
            .bind(account.total_income())
            .execute(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Save account"))?;

        Ok(())
    }

    async fn find_by_id(&self, id: &AccountId) -> Result<Option<Account>, DomainError> {
        let query = "SELECT id, name, provider_id, cookies, api_user, enabled, last_check_in, created_at, auto_checkin_enabled, auto_checkin_hour, auto_checkin_minute, last_login_at, session_token, session_expires_at, last_balance_check_at, current_balance, total_consumed, total_income FROM accounts WHERE id = ?1";

        let row: Option<AccountRow> = sqlx::query_as(query)
            .bind(id.as_str())
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find account by ID"))?;

        match row {
            Some(row) => Ok(Some(row.to_account()?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> Result<Vec<Account>, DomainError> {
        let query = "SELECT id, name, provider_id, cookies, api_user, enabled, last_check_in, created_at, auto_checkin_enabled, auto_checkin_hour, auto_checkin_minute, last_login_at, session_token, session_expires_at, last_balance_check_at, current_balance, total_consumed, total_income FROM accounts ORDER BY created_at DESC";

        let rows: Vec<AccountRow> = sqlx::query_as(query)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find all accounts"))?;

        rows.into_iter().map(|row| row.to_account()).collect()
    }

    async fn find_enabled(&self) -> Result<Vec<Account>, DomainError> {
        let query = "SELECT id, name, provider_id, cookies, api_user, enabled, last_check_in, created_at, auto_checkin_enabled, auto_checkin_hour, auto_checkin_minute, last_login_at, session_token, session_expires_at, last_balance_check_at, current_balance, total_consumed, total_income FROM accounts WHERE enabled = true ORDER BY created_at DESC";

        let rows: Vec<AccountRow> = sqlx::query_as(query)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find enabled accounts"))?;

        rows.into_iter().map(|row| row.to_account()).collect()
    }

    async fn delete(&self, id: &AccountId) -> Result<(), DomainError> {
        let query = "DELETE FROM accounts WHERE id = ?1";

        sqlx::query(query)
            .bind(id.as_str())
            .execute(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Delete account"))?;

        Ok(())
    }
}
