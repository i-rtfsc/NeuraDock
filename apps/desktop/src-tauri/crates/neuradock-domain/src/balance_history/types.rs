use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::{AccountId, DomainError};

#[derive(Debug, Clone)]
pub struct CheckInRecord {
    pub account_id: AccountId,
    pub check_in_date: NaiveDate,
    pub reward_cents: Option<i64>,
    pub source: String,
    pub recorded_at: DateTime<Utc>,
}

impl CheckInRecord {
    pub fn new(
        account_id: AccountId,
        recorded_at: DateTime<Utc>,
        reward: Option<f64>,
        source: &str,
    ) -> Result<Self, DomainError> {
        if reward.is_some_and(|value| !value.is_finite() || !(0.0..=1e12).contains(&value)) {
            return Err(DomainError::Validation(
                "Invalid check-in reward".to_string(),
            ));
        }
        if !matches!(
            source,
            "provider_response"
                | "provider_rule"
                | "confirmed_unknown"
                | "legacy_estimate"
                | "manual"
        ) {
            return Err(DomainError::Validation(
                "Invalid check-in reward source".to_string(),
            ));
        }
        Ok(Self {
            account_id,
            check_in_date: recorded_at.date_naive(),
            reward_cents: reward.map(|value| (value * 100.0).round() as i64),
            source: source.to_string(),
            recorded_at,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceHistoryRecord {
    id: String,
    account_id: AccountId,
    current_balance: f64,
    total_consumed: f64,
    total_quota: f64,
    recorded_at: DateTime<Utc>,
}

impl BalanceHistoryRecord {
    pub fn new(
        id: String,
        account_id: AccountId,
        current_balance: f64,
        total_consumed: f64,
        total_quota: f64,
        recorded_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if id.is_empty() {
            return Err(DomainError::Validation(
                "Balance history id cannot be empty".to_string(),
            ));
        }
        if current_balance < 0.0 {
            return Err(DomainError::Validation(
                "Current balance cannot be negative".to_string(),
            ));
        }
        if total_consumed < 0.0 {
            return Err(DomainError::Validation(
                "Total consumed cannot be negative".to_string(),
            ));
        }
        if total_quota < 0.0 {
            return Err(DomainError::Validation(
                "Total quota cannot be negative".to_string(),
            ));
        }

        Ok(Self {
            id,
            account_id,
            current_balance,
            total_consumed,
            total_quota,
            recorded_at,
        })
    }

    pub fn restore(
        id: String,
        account_id: AccountId,
        current_balance: f64,
        total_consumed: f64,
        total_quota: f64,
        recorded_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            account_id,
            current_balance,
            total_consumed,
            total_quota,
            recorded_at,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    pub fn current_balance(&self) -> f64 {
        self.current_balance
    }

    pub fn total_consumed(&self) -> f64 {
        self.total_consumed
    }

    pub fn total_quota(&self) -> f64 {
        self.total_quota
    }

    pub fn recorded_at(&self) -> DateTime<Utc> {
        self.recorded_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceHistoryDailySummary {
    check_in_date: NaiveDate,
    daily_total_quota: f64,
    daily_balance: f64,
    daily_consumed: f64,
    check_in_reward: Option<f64>,
    check_in_source: Option<String>,
}

impl BalanceHistoryDailySummary {
    pub fn new(
        check_in_date: NaiveDate,
        daily_total_quota: f64,
        daily_balance: f64,
        daily_consumed: f64,
    ) -> Result<Self, DomainError> {
        if daily_total_quota < 0.0 {
            return Err(DomainError::Validation(
                "Daily total income cannot be negative".to_string(),
            ));
        }
        if daily_balance < 0.0 {
            return Err(DomainError::Validation(
                "Daily balance cannot be negative".to_string(),
            ));
        }
        if daily_consumed < 0.0 {
            return Err(DomainError::Validation(
                "Daily consumed cannot be negative".to_string(),
            ));
        }

        Ok(Self {
            check_in_date,
            daily_total_quota,
            daily_balance,
            daily_consumed,
            check_in_reward: None,
            check_in_source: None,
        })
    }

    pub fn restore(
        check_in_date: NaiveDate,
        daily_total_quota: f64,
        daily_balance: f64,
        daily_consumed: f64,
    ) -> Self {
        Self {
            check_in_date,
            daily_total_quota,
            daily_balance,
            daily_consumed,
            check_in_reward: None,
            check_in_source: None,
        }
    }

    pub fn with_check_in(mut self, reward: Option<f64>, source: Option<String>) -> Self {
        self.check_in_reward = reward;
        self.check_in_source = source;
        self
    }

    pub fn is_checked_in(&self) -> bool {
        self.check_in_source.is_some()
    }

    pub fn check_in_reward(&self) -> Option<f64> {
        self.check_in_reward
    }

    pub fn reward_is_estimate(&self) -> bool {
        self.check_in_source.as_deref() == Some("legacy_estimate")
    }

    pub fn check_in_date(&self) -> NaiveDate {
        self.check_in_date
    }

    pub fn daily_total_quota(&self) -> f64 {
        self.daily_total_quota
    }

    pub fn daily_balance(&self) -> f64 {
        self.daily_balance
    }

    pub fn daily_consumed(&self) -> f64 {
        self.daily_consumed
    }
}
