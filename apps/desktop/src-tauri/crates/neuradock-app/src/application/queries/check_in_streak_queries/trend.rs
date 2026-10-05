use chrono::{Duration, NaiveDate, Utc};
use std::collections::HashMap;

use crate::application::dtos::{CheckInDayDto, CheckInTrendDto, TrendDataPoint};
use neuradock_domain::balance_history::{BalanceHistoryDailySummary, BalanceHistoryRepository};
use neuradock_domain::check_in::today_quota_day;
use neuradock_domain::shared::AccountId;
use neuradock_domain::shared::DomainError;

/// Get check-in trend data (last N days).
///
/// Every day in the window gets a point once the first reading is known, with
/// the last known balance carried forward across days that have no snapshot.
/// Returning only the days that happen to have rows left the chart with gaps
/// that looked like data loss.
pub async fn get_trend(
    balance_history_repo: &dyn BalanceHistoryRepository,
    account_id: &str,
    days: u32,
    provider_day_offset_minutes: i32,
) -> Result<CheckInTrendDto, DomainError> {
    if days == 0 || days > 365 {
        return Err(DomainError::Validation(
            "Days must be between 1 and 365".to_string(),
        ));
    }

    let end_date = today_quota_day(provider_day_offset_minutes, Utc::now());
    let start_date = end_date - Duration::days(days as i64 - 1);

    let rows = balance_history_repo
        .list_daily_summaries_in_range(&AccountId::from_string(account_id), start_date, end_date)
        .await?;

    let known: HashMap<NaiveDate, &BalanceHistoryDailySummary> =
        rows.iter().map(|row| (row.quota_day(), row)).collect();

    let mut data_points = Vec::with_capacity(days as usize);
    let mut carried: Option<(f64, f64)> = None;
    let mut cursor = start_date;

    while cursor <= end_date {
        let date = cursor.format("%Y-%m-%d").to_string();

        if let Some(row) = known.get(&cursor) {
            carried = Some((row.daily_total_quota(), row.daily_balance()));
            data_points.push(TrendDataPoint {
                date,
                total_quota: row.daily_total_quota(),
                income_increment: row.check_in_reward(),
                reward_is_estimate: row.reward_is_estimate(),
                current_balance: row.daily_balance(),
                is_checked_in: row.is_checked_in(),
            });
        } else if let Some((total_quota, current_balance)) = carried {
            data_points.push(TrendDataPoint {
                date,
                total_quota,
                income_increment: None,
                reward_is_estimate: false,
                current_balance,
                is_checked_in: false,
            });
        }
        // Days before the first known reading are omitted: there is nothing to
        // carry forward yet, and a zero would be a lie.

        cursor += Duration::days(1);
    }

    let dto = CheckInTrendDto {
        account_id: account_id.to_string(),
        start_date: start_date.format("%Y-%m-%d").to_string(),
        end_date: end_date.format("%Y-%m-%d").to_string(),
        data_points,
    };

    if dto.data_points.is_empty() {
        log::warn!("[streak] trend result empty account_id={}", dto.account_id);
    }

    Ok(dto)
}

/// Get details for a specific day
pub async fn get_day_detail(
    balance_history_repo: &dyn BalanceHistoryRepository,
    account_id: &str,
    date: &str,
) -> Result<CheckInDayDto, DomainError> {
    // Validate date format
    let parsed_date = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| {
        DomainError::Validation("Invalid date format, expected YYYY-MM-DD".to_string())
    })?;

    let row = balance_history_repo
        .find_daily_summary(&AccountId::from_string(account_id), parsed_date)
        .await?;

    if let Some(row) = row {
        // Get previous day's snapshot to calculate increments
        let prev_summary = if let Some(prev_date) = parsed_date.pred_opt() {
            balance_history_repo
                .find_daily_summary(&AccountId::from_string(account_id), prev_date)
                .await?
        } else {
            None
        };

        let income_increment = row.check_in_reward();

        let daily_consumed = prev_summary.as_ref().map_or(0.0, |prev| {
            let diff = row.daily_consumed() - prev.daily_consumed();
            if diff > 0.0 {
                diff
            } else {
                0.0
            }
        });

        let is_checked_in = row.is_checked_in();

        Ok(CheckInDayDto {
            date: date.to_string(),
            is_checked_in,
            income_increment,
            reward_is_estimate: row.reward_is_estimate(),
            current_balance: row.daily_balance(),
            daily_consumed,
            total_consumed: row.daily_consumed(),
            total_quota: row.daily_total_quota(),
        })
    } else {
        log::warn!(
            "[streak] day detail not found account_id={} date={}",
            account_id,
            date
        );
        // No data for this day
        Ok(CheckInDayDto {
            date: date.to_string(),
            is_checked_in: false,
            income_increment: None,
            reward_is_estimate: false,
            current_balance: 0.0,
            daily_consumed: 0.0,
            total_consumed: 0.0,
            total_quota: 0.0,
        })
    }
}
