use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::{AccountId, DomainError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceHistoryRecord {
    id: String,
    account_id: AccountId,
    current_balance: f64,
    total_consumed: f64,
    total_income: f64,
    recorded_at: DateTime<Utc>,
}

impl BalanceHistoryRecord {
    pub fn new(
        id: String,
        account_id: AccountId,
        current_balance: f64,
        total_consumed: f64,
        total_income: f64,
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
        if total_income < 0.0 {
            return Err(DomainError::Validation(
                "Total income cannot be negative".to_string(),
            ));
        }

        Ok(Self {
            id,
            account_id,
            current_balance,
            total_consumed,
            total_income,
            recorded_at,
        })
    }

    pub fn restore(
        id: String,
        account_id: AccountId,
        current_balance: f64,
        total_consumed: f64,
        total_income: f64,
        recorded_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            account_id,
            current_balance,
            total_consumed,
            total_income,
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

    pub fn total_income(&self) -> f64 {
        self.total_income
    }

    pub fn recorded_at(&self) -> DateTime<Utc> {
        self.recorded_at
    }
}

