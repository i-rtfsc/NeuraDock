use crate::application::dtos::BalanceDto;
use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::{debug, warn};

use neuradock_domain::shared::DomainError;

/// Service for managing balance history records
pub struct BalanceHistoryService {
    pool: Arc<SqlitePool>,
}

impl BalanceHistoryService {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    /// Save balance to balance_history table (one record per day, uses deterministic ID to prevent duplicates)
    pub async fn save_balance_history(
        &self,
        account_id: &str,
        balance: &BalanceDto,
    ) -> Result<(), DomainError> {
        let now = Utc::now();
        let date_str = now.format("%Y-%m-%d").to_string();

        // Generate deterministic ID based on account_id and date
        // This ensures the same account on the same day always has the same ID
        let mut hasher = Sha256::new();
        hasher.update(account_id.as_bytes());
        hasher.update(date_str.as_bytes());
        let hash_result = hasher.finalize();
        let id = format!("{:x}", hash_result);

        // Use INSERT OR REPLACE to handle duplicates atomically
        // SQLite will replace the existing record if ID already exists
        sqlx::query(
            "INSERT OR REPLACE INTO balance_history (id, account_id, current_balance, total_consumed, total_income, recorded_at)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(account_id)
        .bind(balance.current_balance)
        .bind(balance.total_consumed)
        .bind(balance.total_income)
        .bind(now)
        .execute(&*self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to save balance history: {e}")))?;

        debug!(
            account_id,
            current_balance = balance.current_balance,
            total_consumed = balance.total_consumed,
            total_income = balance.total_income,
            "Balance history saved/updated"
        );

        Ok(())
    }

    pub async fn get_latest_balance(
        &self,
        account_id: &str,
    ) -> Result<Option<BalanceDto>, DomainError> {
        let result = sqlx::query_as::<_, (f64, f64, f64)>(
            "SELECT current_balance, total_consumed, total_income
             FROM balance_history
             WHERE account_id = ?
             ORDER BY recorded_at DESC
             LIMIT 1",
        )
        .bind(account_id)
        .fetch_optional(&*self.pool)
        .await;

        match result {
            Ok(Some((current_balance, total_consumed, total_income))) => Ok(Some(BalanceDto {
                current_balance,
                total_consumed,
                total_income,
            })),
            Ok(None) => Ok(None),
            Err(e) => {
                warn!(account_id, "Failed to query latest balance history: {}", e);
                Err(DomainError::Infrastructure(format!(
                    "Failed to query balance history: {e}"
                )))
            }
        }
    }
}
