use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BalanceDto {
    pub current_balance: f64,
    pub total_consumed: f64,
    pub total_quota: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ProviderBalanceDto {
    pub provider_id: String,
    pub provider_name: String,
    pub current_balance: f64,
    pub total_consumed: f64,
    pub total_quota: f64,
    pub account_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BalanceStatisticsDto {
    pub providers: Vec<ProviderBalanceDto>,
    pub total_current_balance: f64,
    pub total_consumed: f64,
    pub total_quota: f64,
}
