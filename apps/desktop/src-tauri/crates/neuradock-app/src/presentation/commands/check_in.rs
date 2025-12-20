use crate::application::commands::check_in_commands::*;
use crate::application::commands::command_handler::CommandHandler;
use crate::application::dtos::{
    self, BatchCheckInResult, CheckInHistoryDto, CheckInStatsDto, ExecuteCheckInResult,
    RunningJobDto,
};
use crate::presentation::error::CommandError;
use crate::presentation::state::{CommandHandlers, Queries};
use tauri::State;

/// Execute check-in for a single account
#[tauri::command]
#[specta::specta]
pub async fn execute_check_in(
    account_id: String,
    handlers: State<'_, CommandHandlers>,
) -> Result<ExecuteCheckInResult, CommandError> {
    log::info!(
        "=== execute_check_in command called for account: {} ===",
        account_id
    );

    let command = ExecuteCheckInCommand {
        account_id: account_id.clone(),
    };

    let result = handlers
        .execute_check_in
        .handle(command)
        .await
        .map_err(CommandError::from)?;

    Ok(ExecuteCheckInResult {
        job_id: account_id,
        success: result.success,
        balance: result.balance,
        error: if result.success {
            None
        } else {
            Some(result.message)
        },
    })
}

/// Execute check-in for multiple accounts
#[tauri::command]
#[specta::specta]
pub async fn execute_batch_check_in(
    account_ids: Vec<String>,
    handlers: State<'_, CommandHandlers>,
) -> Result<BatchCheckInResult, CommandError> {
    let command = BatchExecuteCheckInCommand { account_ids };

    let result = handlers
        .batch_execute_check_in
        .handle(command)
        .await
        .map_err(CommandError::from)?;

    // Convert results to DTOs
    let results_dto: Vec<ExecuteCheckInResult> = result
        .results
        .into_iter()
        .map(|r| ExecuteCheckInResult {
            job_id: format!("batch_{}", r.message.len()), // Placeholder
            success: r.success,
            balance: r.balance,
            error: if r.success { None } else { Some(r.message) },
        })
        .collect();

    Ok(BatchCheckInResult {
        total: result.total as i32,
        succeeded: result.succeeded as i32,
        failed: result.failed as i32,
        results: results_dto,
    })
}

/// Stop a running check-in job
#[tauri::command]
#[specta::specta]
pub async fn stop_check_in(job_id: String) -> Result<bool, CommandError> {
    let _ = job_id;
    Err(CommandError::infrastructure("Not implemented yet"))
}

/// Get check-in history for an account
#[tauri::command]
#[specta::specta]
pub async fn get_check_in_history(
    account_id: Option<String>,
    page: i32,
    page_size: i32,
) -> Result<Vec<CheckInHistoryDto>, CommandError> {
    let _ = (account_id, page, page_size);
    Err(CommandError::infrastructure("Not implemented yet"))
}

/// Get check-in statistics for an account
#[tauri::command]
#[specta::specta]
pub async fn get_check_in_stats(
    account_id: Option<String>,
    period: String,
) -> Result<CheckInStatsDto, CommandError> {
    let _ = (account_id, period);
    Err(CommandError::infrastructure("Not implemented yet"))
}

/// Get currently running check-in jobs
#[tauri::command]
#[specta::specta]
pub async fn get_running_jobs() -> Result<Vec<RunningJobDto>, CommandError> {
    Err(CommandError::infrastructure("Not implemented yet"))
}

/// Get check-in streak statistics for an account
#[tauri::command]
#[specta::specta]
pub async fn get_check_in_streak(
    account_id: String,
    queries: State<'_, Queries>,
) -> Result<dtos::CheckInStreakDto, CommandError> {
    queries
        .streak
        .get_streak_stats(&account_id)
        .await
        .map_err(CommandError::from)
}

/// Get check-in streak statistics for all accounts
#[tauri::command]
#[specta::specta]
pub async fn get_all_check_in_streaks(
    queries: State<'_, Queries>,
) -> Result<Vec<dtos::CheckInStreakDto>, CommandError> {
    queries
        .streak
        .get_all_streaks()
        .await
        .map_err(CommandError::from)
}

/// Get check-in calendar for a specific month
#[tauri::command]
#[specta::specta]
pub async fn get_check_in_calendar(
    account_id: String,
    year: i32,
    month: u32,
    queries: State<'_, Queries>,
) -> Result<dtos::CheckInCalendarDto, CommandError> {
    queries
        .streak
        .get_calendar(&account_id, year, month)
        .await
        .map_err(CommandError::from)
}

/// Get check-in trend over a period of days
#[tauri::command]
#[specta::specta]
pub async fn get_check_in_trend(
    account_id: String,
    days: u32,
    queries: State<'_, Queries>,
) -> Result<dtos::CheckInTrendDto, CommandError> {
    queries
        .streak
        .get_trend(&account_id, days)
        .await
        .map_err(CommandError::from)
}

/// Get detailed check-in information for a specific day
#[tauri::command]
#[specta::specta]
pub async fn get_check_in_day_detail(
    account_id: String,
    date: String,
    queries: State<'_, Queries>,
) -> Result<dtos::CheckInDayDto, CommandError> {
    queries
        .streak
        .get_day_detail(&account_id, &date)
        .await
        .map_err(CommandError::from)
}

/// Recalculate check-in streaks for all accounts
#[tauri::command]
#[specta::specta]
pub async fn recalculate_check_in_streaks(
    queries: State<'_, Queries>,
) -> Result<(), CommandError> {
    queries
        .streak
        .recalculate_all_streaks()
        .await
        .map_err(CommandError::from)
}
