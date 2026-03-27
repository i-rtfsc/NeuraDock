use serde::{Deserialize, Serialize};
use specta::Type;

/// DTO for one Codex rate-limit window displayed in the UI
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CodexRateLimitWindowDto {
    pub used_percent: f64,
    pub window_minutes: Option<i64>,
    pub resets_at: Option<String>,
}

/// DTO for a Codex account displayed in the UI
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CodexAccountDto {
    pub id: String,
    pub email: String,
    pub password: Option<String>,
    pub has_password: bool,
    pub has_tokens: bool,
    pub has_payment_session: bool,
    pub account_id: Option<String>,
    pub plan_type: Option<String>,
    pub has_credits: Option<bool>,
    pub is_unlimited: Option<bool>,
    pub credit_balance: Option<String>,
    pub primary_window: Option<CodexRateLimitWindowDto>,
    pub secondary_window: Option<CodexRateLimitWindowDto>,
    pub quota_checked_at: Option<String>,
    pub token_expires_at: Option<String>,
    pub last_refresh_at: Option<String>,
    pub source: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub token_days_remaining: Option<i64>,
    pub is_token_expired: bool,
}

/// Config for single or batch registration
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RegisterConfig {
    pub mode: RegisterMode,
    pub count: Option<u32>,
    pub concurrency: Option<u32>,
    pub min_interval_secs: Option<u64>,
    pub max_interval_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum RegisterMode {
    Single,
    Batch,
}

/// Progress event payload emitted during registration
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RegisterProgressDto {
    pub task_id: String,
    pub email: Option<String>,
    pub status: RegisterTaskStatus,
    pub message: String,
    pub current: u32,
    pub total: u32,
    pub success_count: u32,
    pub fail_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum RegisterTaskStatus {
    Running,
    Success,
    Failed,
    Cancelled,
}

/// Active Codex auth info (read from ~/.codex/auth.json)
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CodexAuthInfoDto {
    pub auth_mode: String,
    pub email: Option<String>,
    pub openai_api_key: Option<String>,
    pub account_id: Option<String>,
    pub last_refresh: String,
    pub has_tokens: bool,
    pub quota: Option<CodexQuotaDto>,
    pub quota_error: Option<String>,
}

/// Quota info fetched from OpenAI Codex API
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CodexQuotaDto {
    pub plan_type: Option<String>,
    pub has_credits: Option<bool>,
    pub is_unlimited: Option<bool>,
    pub credit_balance: Option<String>,
    pub primary_window: Option<CodexRateLimitWindowDto>,
    pub secondary_window: Option<CodexRateLimitWindowDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CodexInboxCodeDto {
    pub email: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CodexPaymentPlanDto {
    Plus,
    Team,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CodexPaymentIntervalDto {
    Month,
    Year,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CodexPaymentLinkRequestDto {
    pub plan_type: CodexPaymentPlanDto,
    pub country: String,
    pub currency: String,
    pub workspace_name: Option<String>,
    pub seat_quantity: Option<u32>,
    pub price_interval: Option<CodexPaymentIntervalDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CodexPaymentLinkDto {
    pub url: String,
    pub session_id: String,
    pub plan_type: CodexPaymentPlanDto,
    pub country: String,
    pub currency: String,
}
