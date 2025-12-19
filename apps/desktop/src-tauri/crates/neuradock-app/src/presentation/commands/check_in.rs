use crate::application::commands::check_in_commands::*;
use crate::application::commands::command_handler::CommandHandler;
use crate::application::dtos::{
    self, BatchCheckInResult, CheckInHistoryDto, CheckInStatsDto, ExecuteCheckInResult,
    RunningJobDto,
};
use crate::presentation::state::AppState;
use tauri::State;

/// Execute check-in for a single account
#[tauri::command]
#[specta::specta]
pub async fn execute_check_in(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<ExecuteCheckInResult, String> {
    log::info!(
        "=== execute_check_in command called for account: {} ===",
        account_id
    );

    let command = ExecuteCheckInCommand {
        account_id: account_id.clone(),
    };

    let result = state
        .command_handlers
        .execute_check_in
        .handle(command)
        .await
        .map_err(|e| e.to_string())?;

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
    state: State<'_, AppState>,
) -> Result<BatchCheckInResult, String> {
    let command = BatchExecuteCheckInCommand { account_ids };

    let result = state
        .command_handlers
        .batch_execute_check_in
        .handle(command)
        .await
        .map_err(|e| e.to_string())?;

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
pub async fn stop_check_in(job_id: String, state: State<'_, AppState>) -> Result<bool, String> {
    Err("Not implemented yet".to_string())
}

/// Get check-in history for an account
#[tauri::command]
#[specta::specta]
pub async fn get_check_in_history(
    account_id: Option<String>,
    page: i32,
    page_size: i32,
    state: State<'_, AppState>,
) -> Result<Vec<CheckInHistoryDto>, String> {
    Err("Not implemented yet".to_string())
}

/// Get check-in statistics for an account
#[tauri::command]
#[specta::specta]
pub async fn get_check_in_stats(
    account_id: Option<String>,
    period: String,
    state: State<'_, AppState>,
) -> Result<CheckInStatsDto, String> {
    Err("Not implemented yet".to_string())
}

/// Get currently running check-in jobs
#[tauri::command]
#[specta::specta]
pub async fn get_running_jobs(state: State<'_, AppState>) -> Result<Vec<RunningJobDto>, String> {
    Err("Not implemented yet".to_string())
}

/// Get check-in streak statistics for an account
#[tauri::command]
#[specta::specta]
pub async fn get_check_in_streak(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<dtos::CheckInStreakDto, String> {
    state
        .streak_queries
        .get_streak_stats(&account_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get check-in streak statistics for all accounts
#[tauri::command]
#[specta::specta]
pub async fn get_all_check_in_streaks(
    state: State<'_, AppState>,
) -> Result<Vec<dtos::CheckInStreakDto>, String> {
    state
        .streak_queries
        .get_all_streaks()
        .await
        .map_err(|e| e.to_string())
}

/// Get check-in calendar for a specific month
#[tauri::command]
#[specta::specta]
pub async fn get_check_in_calendar(
    account_id: String,
    year: i32,
    month: u32,
    state: State<'_, AppState>,
) -> Result<dtos::CheckInCalendarDto, String> {
    state
        .streak_queries
        .get_calendar(&account_id, year, month)
        .await
        .map_err(|e| e.to_string())
}

/// Get check-in trend over a period of days
#[tauri::command]
#[specta::specta]
pub async fn get_check_in_trend(
    account_id: String,
    days: u32,
    state: State<'_, AppState>,
) -> Result<dtos::CheckInTrendDto, String> {
    state
        .streak_queries
        .get_trend(&account_id, days)
        .await
        .map_err(|e| e.to_string())
}

/// Get detailed check-in information for a specific day
#[tauri::command]
#[specta::specta]
pub async fn get_check_in_day_detail(
    account_id: String,
    date: String,
    state: State<'_, AppState>,
) -> Result<dtos::CheckInDayDto, String> {
    state
        .streak_queries
        .get_day_detail(&account_id, &date)
        .await
        .map_err(|e| e.to_string())
}

/// Recalculate check-in streaks for all accounts
#[tauri::command]
#[specta::specta]
pub async fn recalculate_check_in_streaks(state: State<'_, AppState>) -> Result<(), String> {
    state
        .streak_queries
        .recalculate_all_streaks()
        .await
        .map_err(|e| e.to_string())
}
