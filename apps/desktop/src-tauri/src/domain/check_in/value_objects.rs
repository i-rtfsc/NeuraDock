use serde::{Serialize, Deserialize};
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
    pub quota: f64,      // Current balance (当前余额)
    pub used: f64,       // Historical consumption (历史消耗)
    pub remaining: f64,  // Total income (总收益) = quota + used
}

impl Balance {
    pub fn new(quota: f64, used: f64) -> Self {
        Self {
            quota,
            used,
            remaining: quota + used,  // Total income = current balance + historical consumption
        }
    }
}
