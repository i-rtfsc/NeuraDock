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
    /// Lifetime total quota (总额度). On the production path this is carried over
    /// from `UserInfo::total_quota`, which the HTTP layer derives from the raw
    /// `quota + used_quota` unit sum. It is NOT guaranteed to equal
    /// `current_balance + total_consumed` — those are independently rounded to
    /// 2 decimals and summing them compounds their rounding errors.
    pub total_quota: f64,
}

impl Balance {
    /// Test-only convenience constructor: approximates `total_quota` by summing
    /// the two rounded operands. Production code must NOT use this — it should
    /// pass through the raw-unit-derived `UserInfo::total_quota` instead
    /// (see `http::client::user_info`), otherwise rounding drift accumulates
    /// into the persisted balance history.
    pub fn new(current_balance: f64, total_consumed: f64) -> Self {
        Self {
            current_balance,
            total_consumed,
            total_quota: current_balance + total_consumed,
        }
    }

    /// Construct with an explicitly supplied lifetime total. This is the shape
    /// production code should use.
    pub fn with_total_quota(current_balance: f64, total_consumed: f64, total_quota: f64) -> Self {
        Self {
            current_balance,
            total_consumed,
            total_quota,
        }
    }
}
