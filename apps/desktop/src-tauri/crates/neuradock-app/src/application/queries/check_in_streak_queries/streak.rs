use chrono::Utc;
use log::info;

use crate::application::dtos::CheckInStreakDto;
use neuradock_domain::account::AccountRepository;
use neuradock_domain::balance_history::{compute_streak, BalanceHistoryRepository};
use neuradock_domain::check_in::{today_quota_day, ProviderRepository};
use neuradock_domain::shared::DomainError;

use super::helpers;

/// Get streak statistics for a single account.
///
/// "Current" is anchored to the account's quota day: a run that ended two or
/// more days ago is history, however long it was. The arithmetic itself is
/// `compute_streak`, which is unit tested in the domain.
pub async fn get_streak_stats(
    account_repo: &dyn AccountRepository,
    provider_repo: &dyn ProviderRepository,
    balance_history_repo: &dyn BalanceHistoryRepository,
    account_id: &str,
) -> Result<CheckInStreakDto, DomainError> {
    let account_info = helpers::get_account_info(account_repo, provider_repo, account_id).await?;
    let daily_rows = helpers::fetch_all_daily_summaries(balance_history_repo, account_id).await?;
    let today = today_quota_day(account_info.provider_day_offset_minutes, Utc::now());
    let streak = compute_streak(&daily_rows, today);

    let dto = CheckInStreakDto {
        account_id: account_id.to_string(),
        account_name: account_info.name,
        provider_id: account_info.provider_id,
        provider_name: account_info.provider_name,
        current_streak: streak.current,
        longest_streak: streak.longest,
        total_check_in_days: streak.total_days,
        inferred_days: streak.inferred_days,
        last_check_in_date: streak
            .last_check_in_date
            .map(|d| d.format("%Y-%m-%d").to_string()),
    };

    info!(
        "[streak] get_streak_stats account_id={} current={} longest={} total={} inferred={}",
        dto.account_id,
        dto.current_streak,
        dto.longest_streak,
        dto.total_check_in_days,
        dto.inferred_days
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
    let mut results = Vec::with_capacity(accounts.len());

    for account in accounts {
        let daily_rows =
            helpers::fetch_all_daily_summaries(balance_history_repo, &account.account_id).await?;
        let today = today_quota_day(account.provider_day_offset_minutes, Utc::now());
        let streak = compute_streak(&daily_rows, today);

        results.push(CheckInStreakDto {
            account_id: account.account_id,
            account_name: account.name,
            provider_id: account.provider_id,
            provider_name: account.provider_name,
            current_streak: streak.current,
            longest_streak: streak.longest,
            total_check_in_days: streak.total_days,
            inferred_days: streak.inferred_days,
            last_check_in_date: streak
                .last_check_in_date
                .map(|d| d.format("%Y-%m-%d").to_string()),
        });
    }

    results.sort_by_key(|item| std::cmp::Reverse(item.current_streak));

    info!("[streak] get_all_streaks total_accounts={}", results.len());
    Ok(results)
}
