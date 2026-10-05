use crate::application::commands::check_in_commands::*;
use crate::application::commands::command_handler::CommandHandler;
use crate::application::dtos::{
    self, BatchCheckInResult, CheckInHistoryPageDto, CheckInStateDto, CheckInStatsDto,
    ExecuteCheckInResult, RunningJobDto,
};
use crate::presentation::error::CommandError;
use crate::presentation::state::{CommandHandlers, Queries, Repositories, Services};
use neuradock_domain::check_in::CheckInState;
use tauri::State;

/// Record the user's explicit confirmation of an externally completed check-in.
#[tauri::command]
#[specta::specta]
pub async fn confirm_check_in_reward(
    account_id: String,
    date: String,
    repositories: State<'_, Repositories>,
    services: State<'_, Services>,
) -> Result<bool, CommandError> {
    let account = repositories
        .account
        .find_by_id(&neuradock_domain::shared::AccountId::from_string(
            &account_id,
        ))
        .await?
        .ok_or_else(|| CommandError::not_found("Account not found"))?;
    let provider = repositories
        .provider
        .find_by_id(account.provider_id())
        .await?
        .ok_or_else(|| CommandError::not_found("Provider not found"))?;
    services
        .balance_history
        .confirm_fixed_check_in(&account_id, &date, &provider)
        .await
        .map_err(CommandError::from)
}

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
        account_id: result.account_id,
        account_name: result.account_name,
        provider_id: result.provider_id,
        success: result.success,
        balance: result.balance,
        error: if result.success {
            None
        } else {
            Some(result.message)
        },
        outcome: result.outcome.as_str().to_string(),
        error_code: result.error_code,
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
            account_id: r.account_id,
            account_name: r.account_name,
            provider_id: r.provider_id,
            success: r.success,
            balance: r.balance,
            error: if r.success { None } else { Some(r.message) },
            outcome: r.outcome.as_str().to_string(),
            error_code: r.error_code,
        })
        .collect();

    Ok(BatchCheckInResult {
        total: result.total as i32,
        succeeded: result.succeeded as i32,
        failed: result.failed as i32,
        results: results_dto,
    })
}

/// Ask a running batch check-in to stop.
///
/// Cancellation is cooperative: the batch checks between accounts, so a request
/// already in flight finishes. Returns false when the job is unknown, which is
/// what an already-finished run looks like.
#[tauri::command]
#[specta::specta]
pub async fn stop_check_in(
    job_id: String,
    services: State<'_, Services>,
) -> Result<bool, CommandError> {
    Ok(services.jobs.cancel(&job_id))
}

/// Get check-in history (attempts, newest first)
#[tauri::command]
#[specta::specta]
pub async fn get_check_in_history(
    account_id: Option<String>,
    page: i32,
    page_size: i32,
    queries: State<'_, Queries>,
) -> Result<CheckInHistoryPageDto, CommandError> {
    queries
        .streak
        .get_history(account_id.as_deref(), page, page_size)
        .await
        .map_err(CommandError::from)
}

/// Get check-in statistics for one account over the last N quota days
#[tauri::command]
#[specta::specta]
pub async fn get_check_in_stats(
    account_id: String,
    period_days: u32,
    queries: State<'_, Queries>,
) -> Result<CheckInStatsDto, CommandError> {
    queries
        .streak
        .get_stats(&account_id, period_days)
        .await
        .map_err(CommandError::from)
}

/// Get today's check-in state for one account
#[tauri::command]
#[specta::specta]
pub async fn get_check_in_state(
    account_id: String,
    repositories: State<'_, Repositories>,
    services: State<'_, Services>,
) -> Result<CheckInStateDto, CommandError> {
    let account = repositories
        .account
        .find_by_id(&neuradock_domain::shared::AccountId::from_string(
            &account_id,
        ))
        .await?
        .ok_or_else(|| CommandError::not_found("Account not found"))?;
    let provider = repositories
        .provider
        .find_by_id(account.provider_id())
        .await?
        .ok_or_else(|| CommandError::not_found("Provider not found"))?;

    let now = chrono::Utc::now();
    let quota_day =
        neuradock_domain::check_in::today_quota_day(provider.check_in_day_offset_minutes(), now);
    let policy = neuradock_domain::check_in::CheckInPolicy::from_account_min_interval(
        account.check_in_interval_hours(),
    );
    let state = services
        .balance_history
        .check_in_state(&account_id, &provider, now, &policy)
        .await
        .map_err(CommandError::from)?;

    let dto = match state {
        CheckInState::Done { confidence } => CheckInStateDto {
            account_id,
            quota_day: quota_day.to_string(),
            state: "done".to_string(),
            confidence: Some(confidence.as_str().to_string()),
            until: None,
            retryable_failures: 0,
        },
        CheckInState::InFlight { .. } => CheckInStateDto {
            account_id,
            quota_day: quota_day.to_string(),
            state: "in_flight".to_string(),
            confidence: None,
            until: None,
            retryable_failures: 0,
        },
        CheckInState::Backoff {
            until,
            retryable_failures,
        } => CheckInStateDto {
            account_id,
            quota_day: quota_day.to_string(),
            state: "backoff".to_string(),
            confidence: None,
            until: Some(until.to_rfc3339()),
            retryable_failures,
        },
        CheckInState::Due => CheckInStateDto {
            account_id,
            quota_day: quota_day.to_string(),
            state: "due".to_string(),
            confidence: None,
            until: None,
            retryable_failures: 0,
        },
    };

    Ok(dto)
}

/// Get currently running check-in jobs
#[tauri::command]
#[specta::specta]
pub async fn get_running_jobs(
    services: State<'_, Services>,
) -> Result<Vec<RunningJobDto>, CommandError> {
    Ok(services.jobs.list())
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

/// Recompute every account's cached check-in state from the ledger.
///
/// The ledger is the source of truth, so this is a repair operation: it heals
/// drift between the two and returns how many accounts were refreshed.
#[tauri::command]
#[specta::specta]
pub async fn recalculate_check_in_streaks(
    services: State<'_, Services>,
) -> Result<i32, CommandError> {
    services
        .balance_history
        .refresh_all_check_in_caches()
        .await
        .map(|count| count as i32)
        .map_err(CommandError::from)
}
