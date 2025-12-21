use crate::application::dtos::BalanceDto;
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tracing::{debug, warn};

use neuradock_domain::balance_history::{BalanceHistoryRecord, BalanceHistoryRepository};
use neuradock_domain::shared::{AccountId, DomainError};

/// Service for managing balance history records
pub struct BalanceHistoryService {
    repository: Arc<dyn BalanceHistoryRepository>,
}

impl BalanceHistoryService {
    pub fn new(repository: Arc<dyn BalanceHistoryRepository>) -> Self {
        Self { repository }
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

        let record = BalanceHistoryRecord::new(
            id,
            AccountId::from_string(account_id),
            balance.current_balance,
            balance.total_consumed,
            balance.total_income,
            now,
        )?;

        self.repository.save(&record).await?;

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
        match self
            .repository
            .find_latest_by_account_id(&AccountId::from_string(account_id))
            .await
        {
            Ok(Some(record)) => Ok(Some(BalanceDto {
                current_balance: record.current_balance(),
                total_consumed: record.total_consumed(),
                total_income: record.total_income(),
            })),
            Ok(None) => Ok(None),
            Err(e) => {
                warn!(account_id, "Failed to query latest balance history: {}", e);
                Err(e)
            }
        }
    }
}
