use serde::Serialize;
use specta::Type;
use tauri_specta::Event;

#[derive(Serialize, Type, Event, Clone)]
pub struct CheckInProgress {
    pub account_id: String,
    pub progress: f64,
    pub message: String,
}

#[derive(Serialize, Type, Event, Clone)]
pub struct BalanceUpdated {
    pub account_id: String,
    pub current_balance: f64,
    pub total_consumed: f64,
    pub total_income: f64,
}
