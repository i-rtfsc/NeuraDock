use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json;
use sqlx::FromRow;
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, warn};

use crate::domain::account::{Account, AccountRepository, Credentials};
use crate::domain::shared::{AccountId, DomainError, ProviderId};
use crate::infrastructure::persistence::RepositoryErrorMapper;
use crate::infrastructure::security::EncryptionService;

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
    fn to_account(self, encryption: &EncryptionService) -> Result<Account, DomainError> {
        // Try to decrypt cookies JSON, fall back to plaintext if decryption fails
        // This provides backward compatibility with existing unencrypted data
        let cookies_json = match encryption.decrypt(&self.cookies) {
            Ok(decrypted) => decrypted,
            Err(_) => {
                // Assume it's plaintext (legacy data)
                warn!("⚠️  Account {} has unencrypted cookies, will re-encrypt on next save", self.id);
                self.cookies
            }
        };
        
        let cookies = serde_json::from_str(&cookies_json)
            .map_err(|e| RepositoryErrorMapper::map_json_error(e, "Deserialize account cookies"))?;
        
        // Try to decrypt API user, fall back to plaintext if decryption fails
        let api_user = match encryption.decrypt(&self.api_user) {
            Ok(decrypted) => decrypted,
            Err(_) => {
                // Assume it's plaintext (legacy data)
                self.api_user
            }
        };
        
        let credentials = Credentials::new(cookies, api_user);

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
    encryption: Arc<EncryptionService>,
}

impl SqliteAccountRepository {
    pub fn new(pool: Arc<SqlitePool>, encryption: Arc<EncryptionService>) -> Self {
        Self { pool, encryption }
    }
}

#[async_trait]
impl AccountRepository for SqliteAccountRepository {
    async fn save(&self, account: &Account) -> Result<(), DomainError> {
        let start = Instant::now();
        
        // Start a transaction
        let mut tx = self.pool.begin()
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Begin transaction"))?;

        // 1. Save/Update account (without balance/session fields)
        let account_query = r#"
            INSERT INTO accounts (id, name, provider_id, cookies, api_user, enabled, last_check_in, created_at, auto_checkin_enabled, auto_checkin_hour, auto_checkin_minute)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(id) DO UPDATE SET
                name = ?2,
                provider_id = ?3,
                cookies = ?4,
                api_user = ?5,
                enabled = ?6,
                last_check_in = ?7,
                auto_checkin_enabled = ?9,
                auto_checkin_hour = ?10,
                auto_checkin_minute = ?11
        "#;

        // Encrypt cookies JSON
        let cookies_json = serde_json::to_string(account.credentials().cookies())
            .map_err(|e| RepositoryErrorMapper::map_json_error(e, "Serialize account cookies"))?;
        let encrypted_cookies = self.encryption
            .encrypt(&cookies_json)
            .map_err(|e| DomainError::DataIntegrity(format!("Failed to encrypt cookies: {}", e)))?;
        
        // Encrypt API user
        let encrypted_api_user = self.encryption
            .encrypt(account.credentials().api_user())
            .map_err(|e| DomainError::DataIntegrity(format!("Failed to encrypt api_user: {}", e)))?;

        sqlx::query(account_query)
            .bind(account.id().as_str())
            .bind(account.name())
            .bind(account.provider_id().as_str())
            .bind(encrypted_cookies)
            .bind(encrypted_api_user)
            .bind(account.is_enabled())
            .bind(account.last_check_in())
            .bind(account.created_at())
            .bind(account.auto_checkin_enabled())
            .bind(account.auto_checkin_hour() as i64)
            .bind(account.auto_checkin_minute() as i64)
            .execute(&mut *tx)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Save account"))?;

        // 2. Save/Update session if exists
        if let (Some(token), Some(expires_at), Some(last_login_at)) = 
            (account.session_token(), account.session_expires_at(), account.last_login_at()) {
            let session_query = r#"
                INSERT INTO sessions (account_id, token, expires_at, last_login_at)
                VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(account_id) DO UPDATE SET
                    token = ?2,
                    expires_at = ?3,
                    last_login_at = ?4
            "#;
            
            sqlx::query(session_query)
                .bind(account.id().as_str())
                .bind(token)
                .bind(expires_at.to_rfc3339())
                .bind(last_login_at.to_rfc3339())
                .execute(&mut *tx)
                .await
                .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Save session"))?;
        }

        // 3. Save/Update balance if exists
        if let (Some(current), Some(consumed), Some(income), Some(checked_at)) = 
            (account.current_balance(), account.total_consumed(), account.total_income(), account.last_balance_check_at()) {
            let balance_query = r#"
                INSERT INTO balances (account_id, current, total_consumed, total_income, last_checked_at)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(account_id) DO UPDATE SET
                    current = ?2,
                    total_consumed = ?3,
                    total_income = ?4,
                    last_checked_at = ?5
            "#;
            
            sqlx::query(balance_query)
                .bind(account.id().as_str())
                .bind(current)
                .bind(consumed)
                .bind(income)
                .bind(checked_at.to_rfc3339())
                .execute(&mut *tx)
                .await
                .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Save balance"))?;
        }

        // Commit transaction
        tx.commit()
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Commit transaction"))?;

        let elapsed = start.elapsed();
        info!(
            "📊 Account saved: {} in {:.2}ms",
            account.id().as_str(),
            elapsed.as_secs_f64() * 1000.0
        );

        Ok(())
    }

    async fn find_by_id(&self, id: &AccountId) -> Result<Option<Account>, DomainError> {
        let start = Instant::now();
        
        let query = r#"
            SELECT 
                a.id, a.name, a.provider_id, a.cookies, a.api_user, a.enabled, 
                a.last_check_in, a.created_at, a.auto_checkin_enabled, 
                a.auto_checkin_hour, a.auto_checkin_minute,
                s.last_login_at, s.token as session_token, s.expires_at as session_expires_at,
                b.last_checked_at as last_balance_check_at,
                b.current as current_balance,
                b.total_consumed,
                b.total_income
            FROM accounts a
            LEFT JOIN sessions s ON a.id = s.account_id
            LEFT JOIN balances b ON a.id = b.account_id
            WHERE a.id = ?1
        "#;

        let row: Option<AccountRow> = sqlx::query_as(query)
            .bind(id.as_str())
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find account by ID"))?;

        let elapsed = start.elapsed();
        let found = row.is_some();
        info!(
            "📊 find_by_id({}): {:.2}ms, found: {}",
            id.as_str(),
            elapsed.as_secs_f64() * 1000.0,
            found
        );

        match row {
            Some(row) => Ok(Some(row.to_account(&self.encryption)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> Result<Vec<Account>, DomainError> {
        let start = Instant::now();
        
        let query = r#"
            SELECT 
                a.id, a.name, a.provider_id, a.cookies, a.api_user, a.enabled, 
                a.last_check_in, a.created_at, a.auto_checkin_enabled, 
                a.auto_checkin_hour, a.auto_checkin_minute,
                s.last_login_at, s.token as session_token, s.expires_at as session_expires_at,
                b.last_checked_at as last_balance_check_at,
                b.current as current_balance,
                b.total_consumed,
                b.total_income
            FROM accounts a
            LEFT JOIN sessions s ON a.id = s.account_id
            LEFT JOIN balances b ON a.id = b.account_id
            ORDER BY a.created_at DESC
        "#;

        let rows: Vec<AccountRow> = sqlx::query_as(query)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find all accounts"))?;

        let elapsed = start.elapsed();
        let count = rows.len();
        info!(
            "📊 find_all(): {:.2}ms, {} accounts",
            elapsed.as_secs_f64() * 1000.0,
            count
        );
        
        if elapsed.as_millis() > 100 {
            warn!(
                "🐌 SLOW QUERY: find_all() took {:.2}ms for {} accounts",
                elapsed.as_secs_f64() * 1000.0,
                count
            );
        }

        rows.into_iter().map(|row| row.to_account(&self.encryption)).collect()
    }

    async fn find_enabled(&self) -> Result<Vec<Account>, DomainError> {
        let start = Instant::now();
        
        let query = r#"
            SELECT 
                a.id, a.name, a.provider_id, a.cookies, a.api_user, a.enabled, 
                a.last_check_in, a.created_at, a.auto_checkin_enabled, 
                a.auto_checkin_hour, a.auto_checkin_minute,
                s.last_login_at, s.token as session_token, s.expires_at as session_expires_at,
                b.last_checked_at as last_balance_check_at,
                b.current as current_balance,
                b.total_consumed,
                b.total_income
            FROM accounts a
            LEFT JOIN sessions s ON a.id = s.account_id
            LEFT JOIN balances b ON a.id = b.account_id
            WHERE a.enabled = true 
            ORDER BY a.created_at DESC
        "#;

        let rows: Vec<AccountRow> = sqlx::query_as(query)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find enabled accounts"))?;

        let elapsed = start.elapsed();
        let count = rows.len();
        info!(
            "📊 find_enabled(): {:.2}ms, {} accounts",
            elapsed.as_secs_f64() * 1000.0,
            count
        );

        rows.into_iter().map(|row| row.to_account(&self.encryption)).collect()
    }

    async fn delete(&self, id: &AccountId) -> Result<(), DomainError> {
        let start = Instant::now();
        let query = "DELETE FROM accounts WHERE id = ?1";

        sqlx::query(query)
            .bind(id.as_str())
            .execute(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Delete account"))?;

        let elapsed = start.elapsed();
        info!(
            "📊 delete({}): {:.2}ms",
            id.as_str(),
            elapsed.as_secs_f64() * 1000.0
        );

        Ok(())
    }
}
