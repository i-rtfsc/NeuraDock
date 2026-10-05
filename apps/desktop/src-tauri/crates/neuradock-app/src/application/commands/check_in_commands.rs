use crate::application::commands::command_handler::Command;
use crate::application::dtos::BalanceDto;
use neuradock_domain::balance_history::CheckInOutcome;

/// Execute check-in command
#[derive(Debug, Clone)]
pub struct ExecuteCheckInCommand {
    pub account_id: String,
}

impl Command for ExecuteCheckInCommand {}

/// Execute check-in command result
#[derive(Debug, Clone)]
pub struct CheckInCommandResult {
    pub account_id: String,
    pub account_name: String,
    pub provider_id: String,
    pub success: bool,
    pub message: String,
    pub balance: Option<BalanceDto>,
    /// What actually happened, so the UI can tell "checked in" from "already
    /// done today" from "failed" instead of inferring it from `success`.
    pub outcome: CheckInOutcome,
    /// Stable reason code for a failure, for the UI to translate.
    pub error_code: Option<String>,
}

/// Batch execute check-in command
#[derive(Debug, Clone)]
pub struct BatchExecuteCheckInCommand {
    pub account_ids: Vec<String>,
}

impl Command for BatchExecuteCheckInCommand {}

/// Batch execute check-in command result
#[derive(Debug, Clone)]
pub struct BatchCheckInCommandResult {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub results: Vec<CheckInCommandResult>,
}
