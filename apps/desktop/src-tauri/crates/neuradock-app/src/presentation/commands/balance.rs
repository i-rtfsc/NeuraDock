use crate::application::dtos::{BalanceDto, BalanceStatisticsDto, ProviderBalanceDto};
use crate::application::services::CheckInExecutor;
use crate::presentation::state::AppState;
use neuradock_domain::shared::AccountId;
use std::collections::HashMap;
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
        .map_err(|e| e.to_string())?
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
        .map_err(|e| e.to_string())?
        .ok_or(format!("Provider {} not found", provider_id))?;

    // Create executor
    let executor =
        CheckInExecutor::new(state.account_repo.clone(), true).map_err(|e| e.to_string())?;

    // Fetch balance only (without triggering check-in)
    let user_info = executor
        .fetch_balance_only(&account_id, &provider)
        .await
        .map_err(|e| e.to_string())?;

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
        .map_err(|e| e.to_string())?;

    // Store balance history (only if significantly changed or first time)
    save_balance_history(&account_id, &balance_dto, &state).await?;

    Ok(balance_dto)
}

/// Fetch balances for multiple accounts
#[tauri::command]
#[specta::specta]
pub async fn fetch_accounts_balances(
    account_ids: Vec<String>,
    force_refresh: Option<bool>,
    state: State<'_, AppState>,
) -> Result<HashMap<String, Option<BalanceDto>>, String> {
    let mut results = HashMap::new();

    for account_id in account_ids {
        match fetch_account_balance(account_id.clone(), force_refresh, state.clone()).await {
            Ok(balance) => {
                results.insert(account_id, Some(balance));
            }
            Err(_) => {
                results.insert(account_id, None);
            }
        }
    }

    Ok(results)
}

/// Get balance statistics by provider
#[tauri::command]
#[specta::specta]
pub async fn get_balance_statistics(
    state: State<'_, AppState>,
) -> Result<BalanceStatisticsDto, String> {
    let pool = &*state.pool;

    // Get all accounts with their latest balances
    let accounts = state
        .account_repo
        .find_enabled()
        .await
        .map_err(|e| e.to_string())?;
    let providers = state.provider_map().await.map_err(|e| e.to_string())?;

    let mut provider_stats: HashMap<String, ProviderBalanceDto> = HashMap::new();
    let mut total_current_balance = 0.0;
    let mut total_consumed = 0.0;
    let mut total_income = 0.0;

    for account in accounts {
        // Use cached balance from account directly (faster than querying balance_history)
        let balance_opt = match (
            account.current_balance(),
            account.total_consumed(),
            account.total_income(),
        ) {
            (Some(cb), Some(tc), Some(ti)) => Some((cb, tc, ti)),
            _ => {
                // Fallback to balance_history if account cache is empty
                let account_id = account.id().as_str();
                sqlx::query_as::<_, (f64, f64, f64)>(
                    "SELECT current_balance, total_consumed, total_income FROM balance_history
                     WHERE account_id = ? ORDER BY recorded_at DESC LIMIT 1",
                )
                .bind(account_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?
            }
        };

        if let Some((current_balance, consumed, income)) = balance_opt {
            let provider_id = account.provider_id().as_str();
            let provider_name = providers
                .get(provider_id)
                .map(|p| p.name().to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let stat =
                provider_stats
                    .entry(provider_id.to_string())
                    .or_insert(ProviderBalanceDto {
                        provider_id: provider_id.to_string(),
                        provider_name,
                        current_balance: 0.0,
                        total_consumed: 0.0,
                        total_income: 0.0,
                        account_count: 0,
                    });

            stat.current_balance += current_balance;
            stat.total_consumed += consumed;
            stat.total_income += income;
            stat.account_count += 1;

            total_current_balance += current_balance;
            total_consumed += consumed;
            total_income += income;
        }
    }

    Ok(BalanceStatisticsDto {
        providers: provider_stats.into_iter().map(|(_, v)| v).collect(),
        total_current_balance,
        total_consumed,
        total_income,
    })
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
    .map_err(|e| e.to_string())?;

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
            .map_err(|e| e.to_string())?;
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
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
