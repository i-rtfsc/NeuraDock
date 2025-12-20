use crate::application::dtos::BalanceDto;
use crate::application::services::CheckInExecutor;
use crate::application::ResultExt;
use crate::presentation::state::AppState;
use neuradock_domain::shared::AccountId;
use tauri::State;

/// Fetch account balance with smart caching
/// Only performs fresh login if cache is stale (> 1 hour) or no cache exists
/// Set force_refresh to true to ignore cache and always fetch fresh balance
#[tauri::command]
#[specta::specta]
pub async fn fetch_account_balance(
    account_id: String,
    force_refresh: Option<bool>,
    state: State<'_, AppState>,
) -> Result<BalanceDto, String> {
    const MAX_CACHE_AGE_HOURS: i64 = 1;

    let force_refresh = force_refresh.unwrap_or(false);

    // Get account
    let acc_id = AccountId::from_string(&account_id);
    let mut account = state
        .account_repo
        .find_by_id(&acc_id)
        .await
        .to_string_err()?
        .ok_or("Account not found")?;

    // Check if we have valid cached balance (unless force_refresh is true)
    if !force_refresh && !account.is_balance_stale(MAX_CACHE_AGE_HOURS) {
        // Use cached balance
        if let (Some(current_balance), Some(total_consumed), Some(total_income)) = (
            account.current_balance(),
            account.total_consumed(),
            account.total_income(),
        ) {
            return Ok(BalanceDto {
                current_balance,
                total_consumed,
                total_income,
            });
        }
    }

    // Cache is stale or doesn't exist, or force_refresh is true - fetch fresh balance
    let provider_id = account.provider_id().as_str().to_string();
    let provider = state
        .provider_repo
        .find_by_id(account.provider_id())
        .await
        .to_string_err()?
        .ok_or(format!("Provider {} not found", provider_id))?;

    // Create executor
    let executor =
        CheckInExecutor::new(state.account_repo.clone(), true).to_string_err()?;

    // Fetch balance only (without triggering check-in)
    let user_info = executor
        .fetch_balance_only(&account_id, &provider)
        .await
        .to_string_err()?;

    // Note: API returns quota (current balance) and used_quota (total consumed)
    let current_balance = user_info.quota;
    let total_consumed = user_info.used_quota;
    let balance_dto = BalanceDto {
        current_balance,
        total_consumed,
        total_income: current_balance + total_consumed, // Total income = current balance + consumed
    };

    // Update account cache
    account.update_balance(
        balance_dto.current_balance,
        balance_dto.total_consumed,
        balance_dto.total_income,
    );
    state
        .account_repo
        .save(&account)
        .await
        .to_string_err()?;

    // Store balance history (only if significantly changed or first time)
    save_balance_history(&account_id, &balance_dto, &state).await?;

    Ok(balance_dto)
}

/// Helper function to save balance history (one record per day, always update if exists)
async fn save_balance_history(
    account_id: &str,
    balance: &BalanceDto,
    state: &State<'_, AppState>,
) -> Result<(), String> {
    let pool = &*state.pool;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    // Check if we already have a record today
    let today_start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("00:00:00 is always a valid time");
    let today_start_str =
        chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(today_start, chrono::Utc)
            .to_rfc3339();

    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM balance_history WHERE account_id = ? AND recorded_at >= ? LIMIT 1",
    )
    .bind(account_id)
    .bind(&today_start_str)
    .fetch_optional(pool)
    .await
    .to_string_err()?;

    match existing {
        Some((existing_id,)) => {
            // Record exists for today - always update with latest values
            sqlx::query(
                "UPDATE balance_history
                 SET current_balance = ?, total_consumed = ?, total_income = ?, recorded_at = ?
                 WHERE id = ?",
            )
            .bind(balance.current_balance)
            .bind(balance.total_consumed)
            .bind(balance.total_income)
            .bind(now.to_rfc3339())
            .bind(&existing_id)
            .execute(pool)
            .await
            .to_string_err()?;
        }
        None => {
            // No record exists for today - insert new one
            sqlx::query(
                "INSERT INTO balance_history (id, account_id, current_balance, total_consumed, total_income, recorded_at)
                 VALUES (?, ?, ?, ?, ?, ?)"
            )
            .bind(&id)
            .bind(account_id)
            .bind(balance.current_balance)
            .bind(balance.total_consumed)
            .bind(balance.total_income)
            .bind(now.to_rfc3339())
            .execute(pool)
            .await
            .to_string_err()?;
        }
    }

    Ok(())
}
