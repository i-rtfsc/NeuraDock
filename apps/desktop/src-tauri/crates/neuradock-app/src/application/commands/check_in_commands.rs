use crate::application::commands::command_handler::Command;
use crate::application::dtos::BalanceDto;

/// Execute check-in command
#[derive(Debug, Clone)]
pub struct ExecuteCheckInCommand {
    pub account_id: String,
}

impl Command for ExecuteCheckInCommand {}

/// Execute check-in command result
#[derive(Debug, Clone)]
pub struct CheckInCommandResult {
    pub success: bool,
    pub message: String,
    pub balance: Option<BalanceDto>,
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
