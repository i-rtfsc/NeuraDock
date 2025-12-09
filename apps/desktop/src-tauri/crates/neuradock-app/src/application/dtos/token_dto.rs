use neuradock_domain::token::{ApiToken, TokenStatus};

use super::TokenDto;

impl TokenDto {
    pub fn from_domain(token: &ApiToken, account_name: String, provider_name: String) -> Self {
        let status_text = match token.status() {
            TokenStatus::Enabled => "Active",
            TokenStatus::Disabled => "Disabled",
            TokenStatus::Expired => "Expired",
        }
        .to_string();

        let (model_limits_allowed, model_limits_denied) = if let Some(limits) = token.model_limits()
        {
            (limits.allowed.clone(), limits.denied.clone())
        } else {
            (Vec::new(), Vec::new())
        };

        Self {
            id: token.id().value(),
            account_id: token.account_id().to_string(),
            account_name,
            provider_name,
            name: token.name().to_string(),
            key: token.key().to_string(),
            masked_key: token.masked_key(),
            status: token.status().to_i32(),
            status_text,
            used_quota: token.used_quota(),
            remain_quota: token.remain_quota(),
            unlimited_quota: token.unlimited_quota(),
            usage_percentage: token.usage_percentage(),
            expired_time: token.expired_time().map(|dt| dt.timestamp()),
            expired_at: token.expired_time().map(|dt| dt.to_rfc3339()),
            is_active: token.is_active(),
            is_expired: token.is_expired(),
            model_limits_enabled: token.model_limits_enabled(),
            model_limits_allowed,
            model_limits_denied,
            fetched_at: token.fetched_at().to_rfc3339(),
        }
    }
}
