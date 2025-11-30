use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub enum CheckInStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CheckInResult {
    pub success: bool,
    pub balance: Option<Balance>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Balance {
    pub current_balance: f64, // Current balance from API (来自API的当前余额)
    pub total_consumed: f64,  // Total historical consumption from API (来自API的历史消耗)
    pub total_income: f64,    // Total income (总收益) = current_balance + total_consumed
}

impl Balance {
    pub fn new(current_balance: f64, total_consumed: f64) -> Self {
        Self {
            current_balance,
            total_consumed,
            total_income: current_balance + total_consumed, // Total income = current balance + historical consumption
        }
    }
}
