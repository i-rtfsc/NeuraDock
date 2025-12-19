use crate::application::dtos::BalanceDto;
use chrono::Utc;
use log::{error, info};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::sync::Arc;

/// Service for managing balance history records
pub struct BalanceHistoryService {
    pool: Arc<SqlitePool>,
}

impl BalanceHistoryService {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    /// Save balance to balance_history table (one record per day, uses deterministic ID to prevent duplicates)
    pub async fn save_balance_history(&self, account_id: &str, balance: &BalanceDto) {
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
        let result = sqlx::query(
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
        .await;

        match result {
            Ok(_) => {
                info!(
                    "Balance history saved/updated for account {}: current=${:.2}, consumed=${:.2}, income=${:.2}",
                    account_id, balance.current_balance, balance.total_consumed, balance.total_income
                );
            }
            Err(e) => {
                error!("Failed to save/update balance history: {}", e);
            }
        }
    }
}
