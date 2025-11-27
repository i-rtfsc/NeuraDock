use serde::{Serialize, Deserialize};
use specta::Type;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

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
    pub quota: Option<f64>,
    pub used_quota: Option<f64>,
    pub remaining: Option<f64>,
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
    pub quota: f64,
    pub used: f64,
    pub remaining: f64,
}

impl From<Balance> for BalanceDto {
    fn from(b: Balance) -> Self {
        Self {
            quota: b.quota,
            used: b.used,
            remaining: b.remaining,
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
    pub action: String,  // "updated", "created", "failed"
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
    pub quota: f64,
    pub used: f64,
    pub remaining: f64,
    pub account_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BalanceStatisticsDto {
    pub providers: Vec<ProviderBalanceDto>,
    pub total_quota: f64,
    pub total_used: f64,
    pub total_remaining: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BrowserInfoDto {
    pub available: bool,
    pub path: Option<String>,
    pub message: Option<String>,
}
