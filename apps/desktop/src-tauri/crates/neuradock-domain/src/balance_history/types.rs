use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::shared::{AccountId, DomainError};

/// How much the app trusts a check-in evidence row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CheckInConfidence {
    /// The provider (or the user) said so.
    Confirmed,
    /// Derived from a quota increase; can be a top-up rather than a check-in.
    Inferred,
    /// The user explicitly recorded a check-in the app could not observe.
    Manual,
}

impl CheckInConfidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Inferred => "inferred",
            Self::Manual => "manual",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "confirmed" => Ok(Self::Confirmed),
            "inferred" => Ok(Self::Inferred),
            "manual" => Ok(Self::Manual),
            other => Err(DomainError::Validation(format!(
                "Invalid check-in confidence: {other}"
            ))),
        }
    }
}

/// Where a piece of check-in evidence came from.
///
/// Order matters: [`CheckInSource::precedence`] is the ladder the UPSERT guard
/// and the evidence rebuild migration both use, so the three places can never
/// disagree about which row wins for a day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CheckInSource {
    ProviderResponse,
    ProviderRule,
    ConfirmedUnknown,
    LegacyEstimate,
    Manual,
    /// Inferred at runtime from a quota increase between two balance snapshots.
    BalanceDelta,
}

impl CheckInSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ProviderResponse => "provider_response",
            Self::ProviderRule => "provider_rule",
            Self::ConfirmedUnknown => "confirmed_unknown",
            Self::LegacyEstimate => "legacy_estimate",
            Self::Manual => "manual",
            Self::BalanceDelta => "balance_delta",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "provider_response" => Ok(Self::ProviderResponse),
            "provider_rule" => Ok(Self::ProviderRule),
            "confirmed_unknown" => Ok(Self::ConfirmedUnknown),
            "legacy_estimate" => Ok(Self::LegacyEstimate),
            "manual" => Ok(Self::Manual),
            "balance_delta" => Ok(Self::BalanceDelta),
            other => Err(DomainError::Validation(format!(
                "Invalid check-in reward source: {other}"
            ))),
        }
    }

    /// Confidence is derived, never supplied, so the two can never disagree.
    pub fn confidence(&self) -> CheckInConfidence {
        match self {
            Self::Manual => CheckInConfidence::Manual,
            Self::LegacyEstimate | Self::BalanceDelta => CheckInConfidence::Inferred,
            Self::ProviderResponse | Self::ProviderRule | Self::ConfirmedUnknown => {
                CheckInConfidence::Confirmed
            }
        }
    }

    /// Lower wins. Mirrors the migration's `CASE` and the UPSERT guard.
    pub fn precedence(&self) -> u8 {
        match self {
            Self::Manual => 0,
            Self::ProviderResponse => 1,
            Self::ProviderRule => 2,
            Self::ConfirmedUnknown => 3,
            Self::BalanceDelta => 4,
            Self::LegacyEstimate => 5,
        }
    }

    pub fn is_inferred(&self) -> bool {
        matches!(self.confidence(), CheckInConfidence::Inferred)
    }
}

/// One check-in for one quota day. The ledger's primary key is
/// `(account_id, quota_day)`, so a day holds at most one row and later
/// observations promote it rather than adding to it.
#[derive(Debug, Clone)]
pub struct CheckInRecord {
    account_id: AccountId,
    quota_day: NaiveDate,
    reward_cents: Option<i64>,
    source: CheckInSource,
    recorded_at: DateTime<Utc>,
    attempt_id: Option<i64>,
}

impl CheckInRecord {
    pub fn new(
        account_id: AccountId,
        quota_day: NaiveDate,
        recorded_at: DateTime<Utc>,
        reward: Option<f64>,
        source: CheckInSource,
    ) -> Result<Self, DomainError> {
        if reward.is_some_and(|value| !value.is_finite() || !(0.0..=1e12).contains(&value)) {
            return Err(DomainError::Validation(
                "Invalid check-in reward".to_string(),
            ));
        }
        Ok(Self {
            account_id,
            quota_day,
            reward_cents: reward.map(reward_to_cents),
            source,
            recorded_at,
            attempt_id: None,
        })
    }

    /// Rebuild a row read from persistence without re-validating it.
    pub fn restore(
        account_id: AccountId,
        quota_day: NaiveDate,
        reward_cents: Option<i64>,
        source: CheckInSource,
        recorded_at: DateTime<Utc>,
        attempt_id: Option<i64>,
    ) -> Self {
        Self {
            account_id,
            quota_day,
            reward_cents,
            source,
            recorded_at,
            attempt_id,
        }
    }

    pub fn with_attempt_id(mut self, attempt_id: Option<i64>) -> Self {
        self.attempt_id = attempt_id;
        self
    }

    pub fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    pub fn quota_day(&self) -> NaiveDate {
        self.quota_day
    }

    pub fn reward_cents(&self) -> Option<i64> {
        self.reward_cents
    }

    pub fn reward(&self) -> Option<f64> {
        self.reward_cents.map(|cents| cents as f64 / 100.0)
    }

    pub fn source(&self) -> CheckInSource {
        self.source
    }

    pub fn confidence(&self) -> CheckInConfidence {
        self.source.confidence()
    }

    pub fn reward_is_estimate(&self) -> bool {
        self.source.is_inferred()
    }

    pub fn recorded_at(&self) -> DateTime<Utc> {
        self.recorded_at
    }

    pub fn attempt_id(&self) -> Option<i64> {
        self.attempt_id
    }
}

/// Cents are the ledger's storage unit; rounding happens exactly once, here.
pub fn reward_to_cents(reward: f64) -> i64 {
    (reward * 100.0).round() as i64
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
    quota_day: NaiveDate,
    daily_total_quota: f64,
    daily_balance: f64,
    daily_consumed: f64,
    check_in_reward: Option<f64>,
    check_in_source: Option<CheckInSource>,
}

impl BalanceHistoryDailySummary {
    pub fn new(
        quota_day: NaiveDate,
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
            quota_day,
            daily_total_quota,
            daily_balance,
            daily_consumed,
            check_in_reward: None,
            check_in_source: None,
        })
    }

    pub fn restore(
        quota_day: NaiveDate,
        daily_total_quota: f64,
        daily_balance: f64,
        daily_consumed: f64,
    ) -> Self {
        Self {
            quota_day,
            daily_total_quota,
            daily_balance,
            daily_consumed,
            check_in_reward: None,
            check_in_source: None,
        }
    }

    pub fn with_check_in(mut self, reward: Option<f64>, source: Option<CheckInSource>) -> Self {
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

    pub fn check_in_source(&self) -> Option<CheckInSource> {
        self.check_in_source
    }

    pub fn check_in_confidence(&self) -> Option<CheckInConfidence> {
        self.check_in_source.map(|source| source.confidence())
    }

    pub fn reward_is_estimate(&self) -> bool {
        self.check_in_source
            .is_some_and(|source| source.is_inferred())
    }

    pub fn quota_day(&self) -> NaiveDate {
        self.quota_day
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
