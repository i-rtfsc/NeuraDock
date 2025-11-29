use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;

use crate::domain::check_in::Balance;

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
