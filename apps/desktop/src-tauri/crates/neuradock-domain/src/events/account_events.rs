use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::any::Any;

use crate::events::DomainEvent;
use crate::shared::{AccountId, ProviderId};

/// Event fired when an account is created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountCreated {
    pub account_id: AccountId,
    pub name: String,
    pub provider_id: ProviderId,
    pub auto_checkin_enabled: bool,
    pub occurred_at: DateTime<Utc>,
}

impl DomainEvent for AccountCreated {
    fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self
    }
}

/// Event fired when an account is updated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdated {
    pub account_id: AccountId,
    pub name: Option<String>,
    pub credentials_updated: bool,
    pub auto_checkin_config_updated: bool,
    pub occurred_at: DateTime<Utc>,
}

impl DomainEvent for AccountUpdated {
    fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self
    }
}

/// Event fired when an account is deleted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountDeleted {
    pub account_id: AccountId,
    pub name: String,
    pub occurred_at: DateTime<Utc>,
}

impl DomainEvent for AccountDeleted {
    fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self
    }
}

/// Event fired when an account is toggled (enabled/disabled)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountToggled {
    pub account_id: AccountId,
    pub enabled: bool,
    pub occurred_at: DateTime<Utc>,
}

impl DomainEvent for AccountToggled {
    fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self
    }
}

/// Event fired when a check-in is completed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckInCompleted {
    pub account_id: AccountId,
    pub success: bool,
    pub message: String,
    pub balance: Option<CheckInBalance>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckInBalance {
    pub current_balance: f64,
    pub total_consumed: f64,
    pub total_income: f64,
}

impl DomainEvent for CheckInCompleted {
    fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self
    }
}

/// Event fired when balance is updated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceUpdated {
    pub account_id: AccountId,
    pub current_balance: f64,
    pub total_consumed: f64,
    pub total_income: f64,
    pub occurred_at: DateTime<Utc>,
}

impl DomainEvent for BalanceUpdated {
    fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self
    }
}
