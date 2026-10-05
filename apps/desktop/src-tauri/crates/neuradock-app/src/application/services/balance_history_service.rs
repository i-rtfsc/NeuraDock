use crate::application::dtos::BalanceDto;
use chrono::{DateTime, NaiveDate, Utc};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tracing::{debug, warn};

use neuradock_domain::balance_history::{
    BalanceHistoryRecord, BalanceHistoryRepository, CheckInRecord,
};
use neuradock_domain::shared::{AccountId, DomainError};

/// Service for managing balance history records
pub struct BalanceHistoryService {
    repository: Arc<dyn BalanceHistoryRepository>,
}

impl BalanceHistoryService {
    /// The user explicitly confirms a check-in completed outside the app.
    /// This records a reward only; it does not submit a provider request.
    pub async fn confirm_fixed_check_in(
        &self,
        account_id: &str,
        date: &str,
        provider: &neuradock_domain::check_in::Provider,
    ) -> Result<bool, DomainError> {
        let date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| DomainError::Validation("Invalid check-in date".to_string()))?;
        if date > Utc::now().date_naive() {
            return Err(DomainError::Validation(
                "Cannot confirm a future check-in".to_string(),
            ));
        }
        let reward = super::check_in_executor::fixed_reward(provider).ok_or_else(|| {
            DomainError::Validation("This provider has no confirmed fixed reward".to_string())
        })?;
        let id = AccountId::from_string(account_id);
        if self
            .repository
            .find_daily_summary(&id, date)
            .await?
            .is_some_and(|row| {
                row.is_checked_in() && !row.reward_is_estimate() && row.check_in_reward().is_some()
            })
        {
            return Ok(false);
        }
        let mut record = CheckInRecord::new(id, Utc::now(), Some(reward), "manual")?;
        record.check_in_date = date;
        self.repository.save_check_in(&record).await
    }

    pub async fn save_check_in_reward(
        &self,
        account_id: &str,
        recorded_at: DateTime<Utc>,
        reward: Option<f64>,
        source: &str,
    ) -> Result<bool, DomainError> {
        let record = CheckInRecord::new(
            AccountId::from_string(account_id),
            recorded_at,
            reward,
            source,
        )?;
        self.repository.save_check_in(&record).await
    }

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
            balance.total_quota,
            now,
        )?;

        self.repository.save(&record).await?;

        debug!(
            account_id,
            current_balance = balance.current_balance,
            total_consumed = balance.total_consumed,
            total_quota = balance.total_quota,
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
                total_quota: record.total_quota(),
            })),
            Ok(None) => Ok(None),
            Err(e) => {
                warn!(account_id, "Failed to query latest balance history: {}", e);
                Err(e)
            }
        }
    }
}
