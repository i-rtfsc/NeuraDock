use serde::{Deserialize, Serialize};
use specta::Type;

use super::BalanceDto;

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
pub struct ExecuteCheckInResult {
    pub account_id: String,
    pub account_name: String,
    pub provider_id: String,
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
