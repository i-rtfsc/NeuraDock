use chrono::NaiveDate;
use log::{info, warn};

use crate::application::dtos::CheckInStreakDto;
use neuradock_domain::account::AccountRepository;
use neuradock_domain::balance_history::{BalanceHistoryDailySummary, BalanceHistoryRepository};
use neuradock_domain::check_in::ProviderRepository;
use neuradock_domain::shared::DomainError;

use super::helpers;
use super::types::StreakComputation;

/// Get streak statistics for a single account
pub async fn get_streak_stats(
    account_repo: &dyn AccountRepository,
    provider_repo: &dyn ProviderRepository,
    balance_history_repo: &dyn BalanceHistoryRepository,
    account_id: &str,
) -> Result<CheckInStreakDto, DomainError> {
    // Get account metadata and calculate streak from raw data
    let account_info = helpers::get_account_info(account_repo, provider_repo, account_id).await?;
    let daily_rows = helpers::fetch_all_daily_summaries(balance_history_repo, account_id).await?;
    let streak = calculate_streak_stats(account_id, &daily_rows);

    let dto = CheckInStreakDto {
        account_id: account_id.to_string(),
        account_name: account_info.name,
        provider_id: account_info.provider_id,
        provider_name: account_info.provider_name,
        current_streak: streak.current_streak,
        longest_streak: streak.longest_streak,
        total_check_in_days: streak.total_check_in_days,
        last_check_in_date: streak
            .last_check_in_date
            .map(|d| d.format("%Y-%m-%d").to_string()),
    };

    info!(
        "[streak] get_streak_stats account_id={} current={} longest={} total={}",
        dto.account_id, dto.current_streak, dto.longest_streak, dto.total_check_in_days
    );

    Ok(dto)
}

/// Get streak statistics for all accounts
pub async fn get_all_streaks(
    account_repo: &dyn AccountRepository,
    provider_repo: &dyn ProviderRepository,
    balance_history_repo: &dyn BalanceHistoryRepository,
) -> Result<Vec<CheckInStreakDto>, DomainError> {
    let accounts = helpers::get_all_account_infos(account_repo, provider_repo).await?;
    let mut results = Vec::new();

    for account in accounts {
        let daily_rows =
            helpers::fetch_all_daily_summaries(balance_history_repo, &account.account_id).await?;
        let streak = calculate_streak_stats(&account.account_id, &daily_rows);

        results.push(CheckInStreakDto {
            account_id: account.account_id,
            account_name: account.name,
            provider_id: account.provider_id,
            provider_name: account.provider_name,
            current_streak: streak.current_streak,
            longest_streak: streak.longest_streak,
            total_check_in_days: streak.total_check_in_days,
            last_check_in_date: streak
                .last_check_in_date
                .map(|d| d.format("%Y-%m-%d").to_string()),
        });
    }

    results.sort_by_key(|item| std::cmp::Reverse(item.current_streak));

    info!("[streak] get_all_streaks total_accounts={}", results.len());
    Ok(results)
}

/// Recalculate all streaks from balance_history
pub async fn recalculate_all_streaks(
    balance_history_repo: &dyn BalanceHistoryRepository,
) -> Result<(), DomainError> {
    let account_ids = balance_history_repo.list_distinct_account_ids().await?;

    info!(
        "[streak] recalc_all_streaks requested accounts={} (derived on demand)",
        account_ids.len()
    );

    Ok(())
}

fn calculate_streak_stats(
    account_id: &str,
    rows: &[BalanceHistoryDailySummary],
) -> StreakComputation {
    let mut prev_income: Option<f64> = None;
    let mut current_streak = 0u32;
    let mut longest_streak = 0u32;
    let mut total_check_in_days = 0u32;
    let mut last_check_in_date: Option<NaiveDate> = None;

    if rows.is_empty() {
        warn!(
            "[streak] no balance history rows for account_id={}",
            account_id
        );
    }

    for row in rows {
        let date = row.check_in_date();
        let is_checked_in = prev_income.is_none_or(|prev| row.daily_total_quota() > prev);

        if is_checked_in {
            current_streak = match last_check_in_date {
                Some(prev_date) if (date - prev_date).num_days() == 1 => {
                    if current_streak == 0 {
                        1
                    } else {
                        current_streak + 1
                    }
                }
                _ => 1,
            };

            longest_streak = longest_streak.max(current_streak);
            total_check_in_days += 1;
            last_check_in_date = Some(date);
        } else if let Some(prev_date) = last_check_in_date {
            if (date - prev_date).num_days() > 1 {
                current_streak = 0;
            }
        }

        prev_income = Some(row.daily_total_quota());
    }

    StreakComputation {
        current_streak,
        longest_streak,
        total_check_in_days,
        last_check_in_date,
    }
}
