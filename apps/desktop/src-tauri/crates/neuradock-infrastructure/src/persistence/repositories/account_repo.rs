use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json;
use sqlx::FromRow;
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, warn};

use crate::persistence::RepositoryErrorMapper;
use crate::security::EncryptionService;
use neuradock_domain::account::{Account, AccountRepository, Credentials};
use neuradock_domain::shared::{AccountId, DomainError, ProviderId};

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
        // Decrypt cookies JSON - fail if decryption fails (no fallback to plaintext)
        // This ensures all sensitive data is properly encrypted
        let cookies_json = encryption.decrypt(&self.cookies)
            .map_err(|e| {
                DomainError::DataIntegrity(format!(
                    "Failed to decrypt cookies for account {}: {}. Data may be corrupted or using wrong encryption key.",
                    self.id, e
                ))
            })?;

        let cookies = serde_json::from_str(&cookies_json)
            .map_err(|e| RepositoryErrorMapper::map_json_error(e, "Deserialize account cookies"))?;

        // Decrypt API user - fail if decryption fails (no fallback to plaintext)
        let api_user = encryption.decrypt(&self.api_user)
            .map_err(|e| {
                DomainError::DataIntegrity(format!(
                    "Failed to decrypt api_user for account {}: {}. Data may be corrupted or using wrong encryption key.",
                    self.id, e
                ))
            })?;

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
    const SELECT_QUERY: &'static str = r#"
            SELECT
                a.id, a.name, a.provider_id, a.cookies, a.api_user, a.enabled,
                bh.latest_recorded_at as last_check_in, a.created_at, a.auto_checkin_enabled,
                a.auto_checkin_hour, a.auto_checkin_minute,
                s.last_login_at, s.token as session_token, s.expires_at as session_expires_at,
                b.last_checked_at as last_balance_check_at,
                b.current as current_balance,
                b.total_consumed,
                b.total_income
            FROM accounts a
            LEFT JOIN sessions s ON a.id = s.account_id
            LEFT JOIN balances b ON a.id = b.account_id
            LEFT JOIN (
                SELECT account_id, MAX(recorded_at) as latest_recorded_at
                FROM balance_history
                GROUP BY account_id
            ) bh ON a.id = bh.account_id
        "#;

    pub fn new(pool: Arc<SqlitePool>, encryption: Arc<EncryptionService>) -> Self {
        Self { pool, encryption }
    }
}

#[async_trait]
impl AccountRepository for SqliteAccountRepository {
    async fn save(&self, account: &Account) -> Result<(), DomainError> {
        let start = Instant::now();

        // Start a transaction
        let mut tx = self
            .pool
            .begin()
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
        let encrypted_cookies = self
            .encryption
            .encrypt(&cookies_json)
            .map_err(|e| DomainError::DataIntegrity(format!("Failed to encrypt cookies: {}", e)))?;

        // Encrypt API user
        let encrypted_api_user = self
            .encryption
            .encrypt(account.credentials().api_user())
            .map_err(|e| {
                DomainError::DataIntegrity(format!("Failed to encrypt api_user: {}", e))
            })?;

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
        if let (Some(token), Some(expires_at), Some(last_login_at)) = (
            account.session_token(),
            account.session_expires_at(),
            account.last_login_at(),
        ) {
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
        if let (Some(current), Some(consumed), Some(income), Some(checked_at)) = (
            account.current_balance(),
            account.total_consumed(),
            account.total_income(),
            account.last_balance_check_at(),
        ) {
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

        let query = format!(
            r#"
            {}
            WHERE a.id = ?1
        "#,
            Self::SELECT_QUERY
        );

        let row: Option<AccountRow> = sqlx::query_as(&query)
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

    async fn find_by_ids(&self, ids: &[AccountId]) -> Result<Vec<Account>, DomainError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let start = Instant::now();
        let id_strings: Vec<String> = ids.iter().map(|id| id.as_str().to_string()).collect();
        
        // Build parameterized query with placeholders
        let placeholders = (1..=id_strings.len())
            .map(|i| format!("?{}", i))
            .collect::<Vec<_>>()
            .join(",");

        let query = format!(
            r#"
            {}
            WHERE a.id IN ({})
        "#,
            Self::SELECT_QUERY,
            placeholders
        );

        // Build query dynamically with bindings
        let mut query_builder = sqlx::query_as::<_, AccountRow>(&query);
        for id_str in &id_strings {
            query_builder = query_builder.bind(id_str);
        }

        let rows: Vec<AccountRow> = query_builder
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find accounts by IDs"))?;

        let elapsed = start.elapsed();
        info!(
            "📊 find_by_ids({} ids): {:.2}ms, found: {} accounts",
            ids.len(),
            elapsed.as_secs_f64() * 1000.0,
            rows.len()
        );

        rows.into_iter()
            .map(|row| row.to_account(&self.encryption))
            .collect()
    }

    async fn find_all(&self) -> Result<Vec<Account>, DomainError> {
        let start = Instant::now();

        let query = format!(
            r#"
            {}
            ORDER BY a.created_at DESC
        "#,
            Self::SELECT_QUERY
        );

        let rows: Vec<AccountRow> = sqlx::query_as(&query)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find all accounts"))?;

        let elapsed = start.elapsed();
        let count = rows.len();

        if elapsed.as_millis() > 100 {
            warn!(
                "🐌 SLOW QUERY: find_all() took {:.2}ms for {} accounts",
                elapsed.as_secs_f64() * 1000.0,
                count
            );
        }

        // Graceful degradation: if one account fails to load (e.g. decryption error),
        // log it and continue with the others.
        let accounts = rows
            .into_iter()
            .filter_map(|row| match row.to_account(&self.encryption) {
                Ok(account) => Some(account),
                Err(e) => {
                    tracing::error!("Failed to load account: {}", e);
                    None
                }
            })
            .collect();

        info!(
            "📊 find_all(): {:.2}ms, {} accounts loaded",
            elapsed.as_secs_f64() * 1000.0,
            count
        );

        Ok(accounts)
    }

    async fn find_enabled(&self) -> Result<Vec<Account>, DomainError> {
        let start = Instant::now();

        let query = format!(
            r#"
            {}
            WHERE a.enabled = true 
            ORDER BY a.created_at DESC
        "#,
            Self::SELECT_QUERY
        );

        let rows: Vec<AccountRow> = sqlx::query_as(&query)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find enabled accounts"))?;

        let elapsed = start.elapsed();
        let count = rows.len();

        let accounts = rows
            .into_iter()
            .filter_map(|row| match row.to_account(&self.encryption) {
                Ok(account) => Some(account),
                Err(e) => {
                    tracing::error!("Failed to load enabled account: {}", e);
                    None
                }
            })
            .collect();

        info!(
            "📊 find_enabled(): {:.2}ms, {} accounts loaded",
            elapsed.as_secs_f64() * 1000.0,
            count
        );

        Ok(accounts)
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

// Additional utility methods for SqliteAccountRepository
impl SqliteAccountRepository {
    /// Migrate unencrypted accounts by attempting to detect and re-encrypt plaintext data
    ///
    /// ⚠️ WARNING: This method is for migration purposes only and should be called once
    /// during deployment. After migration, all accounts must be properly encrypted.
    ///
    /// Returns the IDs of accounts that were successfully migrated.
    pub async fn migrate_unencrypted_accounts(&self) -> Result<Vec<AccountId>, DomainError> {
        info!("🔄 Starting migration of unencrypted accounts...");

        // Query all raw account rows directly
        let query = "SELECT id, cookies, api_user FROM accounts";
        let rows: Vec<(String, String, String)> = sqlx::query_as(query)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| {
                RepositoryErrorMapper::map_sqlx_error(e, "Fetch accounts for migration")
            })?;

        let mut migrated_ids = Vec::new();
        let mut failed_accounts = Vec::new();

        for (id, cookies, api_user) in rows {
            // Try to decrypt - if it fails, assume it's plaintext and needs migration
            let needs_migration = self.encryption.decrypt(&cookies).is_err()
                || self.encryption.decrypt(&api_user).is_err();

            if needs_migration {
                info!("🔐 Migrating account: {}", id);

                // Assume the data is plaintext and try to encrypt it
                match self.encrypt_account_data(&id, &cookies, &api_user).await {
                    Ok(_) => {
                        migrated_ids.push(AccountId::from_string(&id));
                        info!("✅ Successfully migrated account: {}", id);
                    }
                    Err(e) => {
                        warn!("❌ Failed to migrate account {}: {}", id, e);
                        failed_accounts.push((id.clone(), e.to_string()));
                    }
                }
            }
        }

        if !failed_accounts.is_empty() {
            warn!(
                "⚠️  Migration completed with {} failures:",
                failed_accounts.len()
            );
            for (id, error) in &failed_accounts {
                warn!("  - Account {}: {}", id, error);
            }
        }

        info!(
            "✅ Migration completed. Migrated {} accounts",
            migrated_ids.len()
        );
        Ok(migrated_ids)
    }

    /// Encrypt plaintext account data
    async fn encrypt_account_data(
        &self,
        account_id: &str,
        plaintext_cookies: &str,
        plaintext_api_user: &str,
    ) -> Result<(), DomainError> {
        // Validate that cookies is valid JSON
        let _: serde_json::Value = serde_json::from_str(plaintext_cookies)
            .map_err(|e| DomainError::Validation(format!("Invalid cookies JSON: {}", e)))?;

        // Encrypt the data
        let encrypted_cookies = self
            .encryption
            .encrypt(plaintext_cookies)
            .map_err(|e| DomainError::DataIntegrity(format!("Failed to encrypt cookies: {}", e)))?;

        let encrypted_api_user = self.encryption.encrypt(plaintext_api_user).map_err(|e| {
            DomainError::DataIntegrity(format!("Failed to encrypt api_user: {}", e))
        })?;

        // Update the database
        let update_query = "UPDATE accounts SET cookies = ?1, api_user = ?2 WHERE id = ?3";
        sqlx::query(update_query)
            .bind(encrypted_cookies)
            .bind(encrypted_api_user)
            .bind(account_id)
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                RepositoryErrorMapper::map_sqlx_error(e, "Update encrypted account data")
            })?;

        Ok(())
    }
}
