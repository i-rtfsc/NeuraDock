use serde::{Serialize, Deserialize};
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
    pub quota: f64,
    pub used: f64,
    pub remaining: f64,
}
