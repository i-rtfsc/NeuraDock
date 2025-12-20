use crate::application::dtos::{BalanceStatisticsDto, ProviderBalanceDto};
use crate::presentation::error::CommandError;
use crate::presentation::state::AppState;
use std::collections::HashMap;
use tauri::State;

/// Get balance statistics by provider
#[tauri::command]
#[specta::specta]
pub async fn get_balance_statistics(
    state: State<'_, AppState>,
) -> Result<BalanceStatisticsDto, CommandError> {
    let pool = &*state.pool;

    // Get all accounts with their latest balances
    let accounts = state
        .account_repo
        .find_enabled()
        .await
        .map_err(CommandError::from)?;
    let providers = state.provider_map().await.map_err(CommandError::from)?;

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
                .map_err(CommandError::from)?
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
        providers: provider_stats.into_values().collect(),
        total_current_balance,
        total_consumed,
        total_income,
    })
}
