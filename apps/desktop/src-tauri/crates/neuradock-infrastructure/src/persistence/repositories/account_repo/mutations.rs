use serde_json;
use std::time::Instant;
use tracing::info;

use crate::persistence::RepositoryErrorMapper;
use neuradock_domain::account::Account;
use neuradock_domain::shared::{AccountId, DomainError};

impl super::SqliteAccountRepository {
    pub(super) async fn save_impl(&self, account: &Account) -> Result<(), DomainError> {
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

    pub(super) async fn delete_impl(&self, id: &AccountId) -> Result<(), DomainError> {
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
