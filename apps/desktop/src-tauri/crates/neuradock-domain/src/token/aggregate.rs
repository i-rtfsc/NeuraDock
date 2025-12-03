use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::shared::AccountId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
pub struct TokenId(i64);

impl TokenId {
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    pub fn value(&self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum TokenStatus {
    Enabled = 1,
    Disabled = 2,
    Expired = 3,
}

impl TokenStatus {
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            1 => Some(TokenStatus::Enabled),
            2 => Some(TokenStatus::Disabled),
            3 => Some(TokenStatus::Expired),
            _ => None,
        }
    }

    pub fn to_i32(self) -> i32 {
        self as i32
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ModelLimits {
    pub allowed: Vec<String>,
    pub denied: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ApiToken {
    id: TokenId,
    account_id: AccountId,
    name: String,
    key: String,
    status: TokenStatus,
    used_quota: i64,
    remain_quota: i64,
    unlimited_quota: bool,
    expired_time: Option<DateTime<Utc>>,
    model_limits: Option<ModelLimits>,
    fetched_at: DateTime<Utc>,
}

impl ApiToken {
    pub fn new(
        id: TokenId,
        account_id: AccountId,
        name: String,
        key: String,
        status: TokenStatus,
        used_quota: i64,
        remain_quota: i64,
        unlimited_quota: bool,
        expired_time: Option<DateTime<Utc>>,
        model_limits: Option<ModelLimits>,
    ) -> Self {
        Self {
            id,
            account_id,
            name,
            key,
            status,
            used_quota,
            remain_quota,
            unlimited_quota,
            expired_time,
            model_limits,
            fetched_at: Utc::now(),
        }
    }

    // Getters
    pub fn id(&self) -> &TokenId {
        &self.id
    }

    pub fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn status(&self) -> TokenStatus {
        self.status
    }

    pub fn is_active(&self) -> bool {
        self.status == TokenStatus::Enabled && !self.is_expired()
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expired_time) = self.expired_time {
            expired_time < Utc::now()
        } else {
            false
        }
    }

    pub fn used_quota(&self) -> i64 {
        self.used_quota
    }

    pub fn remain_quota(&self) -> i64 {
        self.remain_quota
    }

    pub fn unlimited_quota(&self) -> bool {
        self.unlimited_quota
    }

    pub fn expired_time(&self) -> Option<DateTime<Utc>> {
        self.expired_time
    }

    pub fn model_limits(&self) -> Option<&ModelLimits> {
        self.model_limits.as_ref()
    }

    pub fn fetched_at(&self) -> DateTime<Utc> {
        self.fetched_at
    }

    pub fn masked_key(&self) -> String {
        if self.key.len() <= 12 {
            return "*".repeat(self.key.len());
        }
        format!("{}...{}", &self.key[..6], &self.key[self.key.len() - 6..])
    }

    // Business logic
    pub fn usage_percentage(&self) -> f64 {
        if self.unlimited_quota {
            return 0.0;
        }
        let total = self.used_quota + self.remain_quota;
        if total == 0 {
            return 0.0;
        }
        (self.used_quota as f64 / total as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_masked_key() {
        let token = ApiToken::new(
            TokenId::new(1),
            AccountId::from("acc-1".to_string()),
            "test".to_string(),
            "sk-123456789012".to_string(),
            TokenStatus::Enabled,
            0,
            100000,
            false,
            None,
            None,
        );

        assert_eq!(token.masked_key(), "sk-123...9012");
    }

    #[test]
    fn test_token_is_expired() {
        let expired_time = Utc::now() - chrono::Duration::hours(1);
        let token = ApiToken::new(
            TokenId::new(1),
            AccountId::from("acc-1".to_string()),
            "test".to_string(),
            "sk-test".to_string(),
            TokenStatus::Enabled,
            0,
            100000,
            false,
            Some(expired_time),
            None,
        );

        assert!(token.is_expired());
        assert!(!token.is_active());
    }

    #[test]
    fn test_usage_percentage() {
        let token = ApiToken::new(
            TokenId::new(1),
            AccountId::from("acc-1".to_string()),
            "test".to_string(),
            "sk-test".to_string(),
            TokenStatus::Enabled,
            30000,
            70000,
            false,
            None,
            None,
        );

        assert_eq!(token.usage_percentage(), 30.0);
    }

    #[test]
    fn test_unlimited_quota_usage_percentage() {
        let token = ApiToken::new(
            TokenId::new(1),
            AccountId::from("acc-1".to_string()),
            "test".to_string(),
            "sk-test".to_string(),
            TokenStatus::Enabled,
            30000,
            70000,
            true,
            None,
            None,
        );

        assert_eq!(token.usage_percentage(), 0.0);
    }
}
