mod repository;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::{AccountId, DomainError};

pub use repository::BalanceRepository;

/// Balance value object
/// Represents account balance information
/// 
/// Separated from Account aggregate because balance has different
/// lifecycle and update frequency. Balance changes frequently during
/// check-ins and balance queries, while account credentials remain stable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    account_id: AccountId,
    current: f64,
    total_consumed: f64,
    total_income: f64,
    last_checked_at: DateTime<Utc>,
}

impl Balance {
    /// Create a new balance record
    pub fn new(account_id: AccountId, current: f64) -> Result<Self, DomainError> {
        if current < 0.0 {
            return Err(DomainError::Validation(
                "Balance cannot be negative".to_string(),
            ));
        }

        Ok(Self {
            account_id,
            current,
            total_consumed: 0.0,
            total_income: 0.0,
            last_checked_at: Utc::now(),
        })
    }

    /// Restore balance from persistence
    pub fn restore(
        account_id: AccountId,
        current: f64,
        total_consumed: f64,
        total_income: f64,
        last_checked_at: DateTime<Utc>,
    ) -> Self {
        Self {
            account_id,
            current,
            total_consumed,
            total_income,
            last_checked_at,
        }
    }

    /// Update current balance
    /// This should be called after querying the provider's API
    pub fn update_current(&mut self, new_balance: f64) -> Result<(), DomainError> {
        if new_balance < 0.0 {
            return Err(DomainError::Validation(
                "Balance cannot be negative".to_string(),
            ));
        }

        self.current = new_balance;
        self.last_checked_at = Utc::now();

        Ok(())
    }

    /// Record consumption (e.g., data usage, service fee)
    pub fn record_consumption(&mut self, amount: f64) -> Result<(), DomainError> {
        if amount < 0.0 {
            return Err(DomainError::Validation(
                "Consumption amount cannot be negative".to_string(),
            ));
        }

        self.total_consumed += amount;
        self.last_checked_at = Utc::now();

        Ok(())
    }

    /// Record income (e.g., check-in reward, top-up)
    pub fn record_income(&mut self, amount: f64) -> Result<(), DomainError> {
        if amount < 0.0 {
            return Err(DomainError::Validation(
                "Income amount cannot be negative".to_string(),
            ));
        }

        self.total_income += amount;
        self.current += amount;
        self.last_checked_at = Utc::now();

        Ok(())
    }

    /// Check if balance is stale (not checked recently)
    pub fn is_stale(&self, hours_threshold: i64) -> bool {
        let threshold = Utc::now() - Duration::hours(hours_threshold);
        self.last_checked_at < threshold
    }

    /// Check if balance is low (below threshold)
    pub fn is_low(&self, threshold: f64) -> bool {
        self.current < threshold
    }

    // Getters
    pub fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    pub fn current(&self) -> f64 {
        self.current
    }

    pub fn total_consumed(&self) -> f64 {
        self.total_consumed
    }

    pub fn total_income(&self) -> f64 {
        self.total_income
    }

    pub fn last_checked_at(&self) -> DateTime<Utc> {
        self.last_checked_at
    }

    /// Calculate net change (income - consumed)
    pub fn net_change(&self) -> f64 {
        self.total_income - self.total_consumed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_balance() -> Balance {
        let account_id = AccountId::new();
        Balance::new(account_id, 100.0).unwrap()
    }

    #[test]
    fn test_create_balance() {
        let balance = create_test_balance();
        assert_eq!(balance.current(), 100.0);
        assert_eq!(balance.total_consumed(), 0.0);
        assert_eq!(balance.total_income(), 0.0);
    }

    #[test]
    fn test_negative_balance_rejected() {
        let account_id = AccountId::new();
        let result = Balance::new(account_id, -10.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_current_balance() {
        let mut balance = create_test_balance();
        balance.update_current(150.0).unwrap();
        assert_eq!(balance.current(), 150.0);
    }

    #[test]
    fn test_record_consumption() {
        let mut balance = create_test_balance();
        balance.record_consumption(20.0).unwrap();
        assert_eq!(balance.total_consumed(), 20.0);
        assert_eq!(balance.current(), 100.0); // Current doesn't change automatically
    }

    #[test]
    fn test_record_income() {
        let mut balance = create_test_balance();
        balance.record_income(50.0).unwrap();
        assert_eq!(balance.total_income(), 50.0);
        assert_eq!(balance.current(), 150.0); // Current increases with income
    }

    #[test]
    fn test_net_change() {
        let mut balance = create_test_balance();
        balance.record_income(100.0).unwrap();
        balance.record_consumption(30.0).unwrap();
        assert_eq!(balance.net_change(), 70.0); // 100 - 30
    }

    #[test]
    fn test_is_low() {
        let balance = create_test_balance();
        assert!(balance.is_low(150.0)); // 100 < 150
        assert!(!balance.is_low(50.0)); // 100 > 50
    }

    #[test]
    fn test_is_stale() {
        let account_id = AccountId::new();
        let last_checked = Utc::now() - Duration::hours(25);
        let balance = Balance::restore(account_id, 100.0, 0.0, 0.0, last_checked);

        assert!(balance.is_stale(24)); // Stale if > 24 hours
        assert!(!balance.is_stale(48)); // Not stale if < 48 hours
    }
}
