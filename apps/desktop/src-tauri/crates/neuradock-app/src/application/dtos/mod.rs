use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;

use neuradock_domain::account::Account;
use neuradock_domain::check_in::Balance;

pub mod token_dto;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AccountDto {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    pub provider_name: String,
    pub enabled: bool,
    pub last_check_in: Option<String>,
    pub created_at: String,
    pub auto_checkin_enabled: bool,
    pub auto_checkin_hour: u8,
    pub auto_checkin_minute: u8,
    pub last_balance_check_at: Option<String>,
    pub current_balance: Option<f64>,
    pub total_consumed: Option<f64>,
    pub total_income: Option<f64>,
    pub is_balance_stale: bool,
    pub is_online: bool,
    // Session expiration info for frontend display
    pub session_expires_at: Option<String>,
    pub session_expires_soon: bool, // true if session expires within 7 days
    pub session_days_remaining: Option<i64>, // days until session expires
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AccountDetailDto {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    pub provider_name: String,
    pub api_user: String,
    pub cookies: HashMap<String, String>,
    pub cookies_count: i32,
    pub enabled: bool,
    pub last_check_in: Option<String>,
    pub last_balance: Option<BalanceDto>,
    pub created_at: String,
    pub auto_checkin_enabled: bool,
    pub auto_checkin_hour: u8,
    pub auto_checkin_minute: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BalanceDto {
    pub current_balance: f64,
    pub total_consumed: f64,
    pub total_income: f64,
}

impl From<Balance> for BalanceDto {
    fn from(b: Balance) -> Self {
        Self {
            current_balance: b.current_balance,
            total_consumed: b.total_consumed,
            total_income: b.total_income,
        }
    }
}

// ============================================================
// Account DTO Conversions
// ============================================================

/// Helper struct for Account -> AccountDto conversion
/// Provides provider name which is not part of the domain model
pub struct AccountDtoMapper<'a> {
    pub provider_name: String,
    pub now: DateTime<Utc>,
    account: &'a Account,
}

impl<'a> AccountDtoMapper<'a> {
    pub fn new(account: &'a Account, provider_name: String) -> Self {
        Self {
            provider_name,
            now: Utc::now(),
            account,
        }
    }

    pub fn with_time(mut self, now: DateTime<Utc>) -> Self {
        self.now = now;
        self
    }

    pub fn to_dto(self) -> AccountDto {
        let acc = self.account;

        // Check if balance is stale (> 24 hours old)
        let is_balance_stale = acc.is_balance_stale(24);

        // Consider account "online" if session is valid OR balance check is recent
        let is_online = acc.is_session_valid() || !is_balance_stale;

        // Calculate session expiration info
        let session_expires_at = acc.session_expires_at();
        let (session_expires_soon, session_days_remaining) = match session_expires_at {
            Some(expires_at) => {
                let duration = expires_at.signed_duration_since(self.now);
                let days_remaining = duration.num_days();
                let expires_soon = days_remaining <= 7;
                (expires_soon, Some(days_remaining.max(0)))
            }
            None => (false, None),
        };

        AccountDto {
            id: acc.id().as_str().to_string(),
            name: acc.name().to_string(),
            provider_id: acc.provider_id().as_str().to_string(),
            provider_name: self.provider_name,
            enabled: acc.is_enabled(),
            last_check_in: acc.last_check_in().map(|dt| dt.to_rfc3339()),
            created_at: acc.created_at().to_rfc3339(),
            auto_checkin_enabled: acc.auto_checkin_enabled(),
            auto_checkin_hour: acc.auto_checkin_hour(),
            auto_checkin_minute: acc.auto_checkin_minute(),
            last_balance_check_at: acc.last_balance_check_at().map(|dt| dt.to_rfc3339()),
            current_balance: acc.current_balance(),
            total_consumed: acc.total_consumed(),
            total_income: acc.total_income(),
            is_balance_stale,
            is_online,
            session_expires_at: session_expires_at.map(|dt| dt.to_rfc3339()),
            session_expires_soon,
            session_days_remaining,
        }
    }
}

/// Helper for AccountDetailDto conversion
pub struct AccountDetailDtoMapper<'a> {
    pub provider_name: String,
    account: &'a Account,
    pub last_balance: Option<BalanceDto>,
}

impl<'a> AccountDetailDtoMapper<'a> {
    pub fn new(account: &'a Account, provider_name: String) -> Self {
        Self {
            provider_name,
            account,
            last_balance: None,
        }
    }

    pub fn with_balance(mut self, balance: Option<BalanceDto>) -> Self {
        self.last_balance = balance;
        self
    }

    pub fn to_dto(self) -> AccountDetailDto {
        let acc = self.account;

        AccountDetailDto {
            id: acc.id().as_str().to_string(),
            name: acc.name().to_string(),
            provider_id: acc.provider_id().as_str().to_string(),
            provider_name: self.provider_name,
            api_user: acc.credentials().api_user().to_string(),
            cookies: acc.credentials().cookies().clone(),
            cookies_count: acc.credentials().cookies().len() as i32,
            enabled: acc.is_enabled(),
            last_check_in: acc.last_check_in().map(|dt| dt.to_rfc3339()),
            last_balance: self.last_balance,
            created_at: acc.created_at().to_rfc3339(),
            auto_checkin_enabled: acc.auto_checkin_enabled(),
            auto_checkin_hour: acc.auto_checkin_hour(),
            auto_checkin_minute: acc.auto_checkin_minute(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ProviderDto {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub is_builtin: bool,
    pub account_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CheckInHistoryDto {
    pub job_id: String,
    pub account_id: String,
    pub account_name: String,
    pub provider_name: String,
    pub status: String,
    pub success: bool,
    pub balance: Option<BalanceDto>,
    pub error: Option<String>,
    pub scheduled_at: String,
    pub executed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CheckInStatsDto {
    pub total_checks: i32,
    pub successful_checks: i32,
    pub failed_checks: i32,
    pub success_rate: f64,
    pub average_balance: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RunningJobDto {
    pub job_id: String,
    pub account_id: String,
    pub account_name: String,
    pub status: String,
    pub started_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CreateAccountInput {
    pub name: String,
    pub provider_id: String,
    pub cookies: HashMap<String, String>,
    pub api_user: String,
    pub auto_checkin_enabled: Option<bool>,
    pub auto_checkin_hour: Option<u8>,
    pub auto_checkin_minute: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UpdateAccountInput {
    pub account_id: String,
    pub name: Option<String>,
    pub cookies: Option<HashMap<String, String>>,
    pub api_user: Option<String>,
    pub auto_checkin_enabled: Option<bool>,
    pub auto_checkin_hour: Option<u8>,
    pub auto_checkin_minute: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ImportAccountInput {
    pub name: String,
    pub provider: String,
    pub cookies: HashMap<String, String>,
    pub api_user: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BatchImportResult {
    pub total: i32,
    pub succeeded: i32,
    pub failed: i32,
    pub results: Vec<ImportItemResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ImportItemResult {
    pub success: bool,
    pub account_id: Option<String>,
    pub account_name: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BatchUpdateResult {
    pub total: i32,
    pub updated: i32,
    pub created: i32,
    pub failed: i32,
    pub results: Vec<UpdateItemResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UpdateItemResult {
    pub success: bool,
    pub account_id: Option<String>,
    pub account_name: String,
    pub action: String, // "updated", "created", "failed"
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ExportAccountsInput {
    pub account_ids: Vec<String>,
    pub include_credentials: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ExecuteCheckInResult {
    pub job_id: String,
    pub success: bool,
    pub balance: Option<BalanceDto>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BatchCheckInResult {
    pub total: i32,
    pub succeeded: i32,
    pub failed: i32,
    pub results: Vec<ExecuteCheckInResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AddProviderInput {
    pub name: String,
    pub domain: String,
    pub login_path: String,
    pub sign_in_path: Option<String>,
    pub user_info_path: String,
    pub api_user_key: String,
    pub bypass_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ProviderBalanceDto {
    pub provider_id: String,
    pub provider_name: String,
    pub current_balance: f64,
    pub total_consumed: f64,
    pub total_income: f64,
    pub account_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BalanceStatisticsDto {
    pub providers: Vec<ProviderBalanceDto>,
    pub total_current_balance: f64,
    pub total_consumed: f64,
    pub total_income: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BrowserInfoDto {
    pub available: bool,
    pub path: Option<String>,
    pub message: Option<String>,
}

// ============================================================
// Check-in Streak DTOs
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CheckInStreakDto {
    pub account_id: String,
    pub account_name: String,
    pub provider_id: String,
    pub provider_name: String,
    pub current_streak: u32,
    pub longest_streak: u32,
    pub total_check_in_days: u32,
    pub last_check_in_date: Option<String>, // ISO 8601 date (YYYY-MM-DD)
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CheckInDayDto {
    pub date: String, // YYYY-MM-DD
    pub is_checked_in: bool,
    pub income_increment: Option<f64>,
    pub current_balance: f64,
    pub total_consumed: f64,
    pub total_income: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CheckInCalendarDto {
    pub account_id: String,
    pub year: i32,
    pub month: u32,
    pub days: Vec<CheckInDayDto>,
    pub month_stats: MonthStatsDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MonthStatsDto {
    pub total_days: u32,
    pub checked_in_days: u32,
    pub check_in_rate: f64, // 签到率百分比 (0.0 - 100.0)
    pub total_income_increment: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CheckInTrendDto {
    pub account_id: String,
    pub start_date: String,
    pub end_date: String,
    pub data_points: Vec<TrendDataPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TrendDataPoint {
    pub date: String,
    pub total_income: f64,
    pub income_increment: f64,
    pub current_balance: f64,
    pub is_checked_in: bool,
}

// ============================================================
// Notification DTOs
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct NotificationChannelDto {
    pub id: String,
    pub channel_type: String,
    #[specta(type = String)]
    pub config: serde_json::Value,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CreateNotificationChannelInput {
    pub channel_type: String,
    #[specta(type = String)]
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UpdateNotificationChannelInput {
    pub channel_id: String,
    #[specta(type = String)]
    pub config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
}

// ============================================================
// Token Management DTOs
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TokenDto {
    pub id: i64,
    pub account_id: String,
    pub account_name: String,
    pub provider_name: String,
    pub name: String,
    pub key: String,
    pub masked_key: String,
    pub status: i32,
    pub status_text: String,
    pub used_quota: i64,
    pub remain_quota: i64,
    pub unlimited_quota: bool,
    pub usage_percentage: f64,
    pub expired_time: Option<i64>,
    pub expired_at: Option<String>,
    pub is_active: bool,
    pub is_expired: bool,
    pub model_limits_enabled: bool,
    pub model_limits_allowed: Vec<String>,
    pub model_limits_denied: Vec<String>,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ProviderNodeDto {
    pub id: String,
    pub name: String,
    pub base_url: String,
}
