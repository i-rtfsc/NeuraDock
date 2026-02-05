use serde::{Deserialize, Serialize};
use specta::Type;

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
    pub daily_consumed: f64,
    pub total_consumed: f64,
    pub total_quota: f64,
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
    pub total_quota_increment: f64,
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
    pub total_quota: f64,
    pub income_increment: f64,
    pub current_balance: f64,
    pub is_checked_in: bool,
}
