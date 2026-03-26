use neuradock_domain::codex::{CodexAccount, CodexRateLimitWindow};
use crate::application::dtos::{CodexAccountDto, CodexRateLimitWindowDto};

fn map_window(window: Option<&CodexRateLimitWindow>) -> Option<CodexRateLimitWindowDto> {
    window.map(|window| CodexRateLimitWindowDto {
        used_percent: window.used_percent(),
        window_minutes: window.window_minutes(),
        resets_at: window.resets_at().map(|dt| dt.to_rfc3339()),
    })
}

impl From<&CodexAccount> for CodexAccountDto {
    fn from(account: &CodexAccount) -> Self {
        Self {
            id: account.id().as_str().to_string(),
            email: account.email().to_string(),
            password: account.password().map(|s| s.to_string()),
            has_password: account.password().is_some(),
            has_tokens: account.access_token().is_some(),
            has_payment_session: account.has_payment_session(),
            account_id: account.account_id().map(|s| s.to_string()),
            plan_type: account.plan_type().map(|s| s.to_string()),
            has_credits: account.has_credits(),
            is_unlimited: account.is_unlimited(),
            credit_balance: account.credit_balance().map(|s| s.to_string()),
            primary_window: map_window(account.primary_window()),
            secondary_window: map_window(account.secondary_window()),
            quota_checked_at: account.quota_checked_at().map(|dt| dt.to_rfc3339()),
            token_expires_at: account.token_expires_at().map(|dt| dt.to_rfc3339()),
            last_refresh_at: account.last_refresh_at().map(|dt| dt.to_rfc3339()),
            source: account.source().as_str().to_string(),
            status: account.status().as_str().to_string(),
            created_at: account.created_at().to_rfc3339(),
            updated_at: account.updated_at().to_rfc3339(),
            token_days_remaining: account.token_days_remaining(),
            is_token_expired: account.is_token_expired(),
        }
    }
}
