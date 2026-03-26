use serde::Serialize;
use specta::Type;
use tauri_specta::Event;
use crate::application::dtos::RegisterTaskStatus;

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
    pub total_quota: f64,
}

#[derive(Serialize, Type, Event, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CodexRegisterProgress {
    pub task_id: String,
    pub email: Option<String>,
    pub status: RegisterTaskStatus,
    pub message: String,
    pub current: u32,
    pub total: u32,
    pub success_count: u32,
    pub fail_count: u32,
}
