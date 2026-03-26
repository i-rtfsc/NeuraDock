use crate::application::dtos::{CodexQuotaDto, CodexRateLimitWindowDto};
use neuradock_domain::codex::{CodexAccount, CodexRateLimitWindow};
use neuradock_infrastructure::http::openai::quota::{CodexUsageQuota, CodexUsageWindow};

pub fn apply_usage_quota(account: &mut CodexAccount, quota: &CodexUsageQuota) {
    account.apply_quota(
        quota.plan_type.clone(),
        quota.has_credits,
        quota.is_unlimited,
        quota.credit_balance.clone(),
        map_domain_window(quota.primary_window.as_ref()),
        map_domain_window(quota.secondary_window.as_ref()),
    );
}

pub fn quota_to_dto(quota: &CodexUsageQuota) -> CodexQuotaDto {
    CodexQuotaDto {
        plan_type: quota.plan_type.clone(),
        has_credits: quota.has_credits,
        is_unlimited: quota.is_unlimited,
        credit_balance: quota.credit_balance.clone(),
        primary_window: map_dto_window(quota.primary_window.as_ref()),
        secondary_window: map_dto_window(quota.secondary_window.as_ref()),
    }
}

fn map_domain_window(window: Option<&CodexUsageWindow>) -> Option<CodexRateLimitWindow> {
    window.map(|window| {
        CodexRateLimitWindow::new(window.used_percent, window.window_minutes, window.resets_at)
    })
}

fn map_dto_window(window: Option<&CodexUsageWindow>) -> Option<CodexRateLimitWindowDto> {
    window.map(|window| CodexRateLimitWindowDto {
        used_percent: window.used_percent,
        window_minutes: window.window_minutes,
        resets_at: window.resets_at.map(|dt| dt.to_rfc3339()),
    })
}
