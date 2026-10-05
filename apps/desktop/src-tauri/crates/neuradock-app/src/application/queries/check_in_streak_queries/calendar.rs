use chrono::{Datelike, NaiveDate, Utc};
use log::{info, warn};
use std::collections::HashMap;

use crate::application::dtos::{CheckInCalendarDto, CheckInDayDto, MonthStatsDto};
use neuradock_domain::balance_history::{BalanceHistoryDailySummary, BalanceHistoryRepository};
use neuradock_domain::check_in::today_quota_day;
use neuradock_domain::shared::{AccountId, DomainError};

/// Get check-in calendar for a specific month.
///
/// `provider_day_offset_minutes` decides where "today" is, which matters twice:
/// the sign-in rate is measured against the days that have already started, and
/// the first day of the month needs the previous day as its consumption
/// baseline.
pub async fn get_calendar(
    balance_history_repo: &dyn BalanceHistoryRepository,
    account_id: &str,
    year: i32,
    month: u32,
    provider_day_offset_minutes: i32,
) -> Result<CheckInCalendarDto, DomainError> {
    // Validate inputs
    if !(1..=12).contains(&month) {
        return Err(DomainError::Validation("Invalid month".to_string()));
    }

    // Calculate the first and last day of the month
    let first_day = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| DomainError::Validation("Invalid date".to_string()))?;

    let first_day_next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    let last_day = first_day_next_month
        .and_then(|d| d.pred_opt())
        .ok_or_else(|| DomainError::Validation("Invalid date".to_string()))?;

    let account = AccountId::from_string(account_id);
    let today = today_quota_day(provider_day_offset_minutes, Utc::now());
    let total_days = last_day.day();

    // Query daily summaries for this month
    let rows = balance_history_repo
        .list_daily_summaries_in_range(&account, first_day, last_day)
        .await?;

    if rows.is_empty() {
        warn!(
            "[streak] calendar query empty result account_id={} month={:04}-{:02}",
            account_id, year, month
        );
    } else {
        info!(
            "[streak] calendar query account_id={} month={:04}-{:02} rows={}",
            account_id,
            year,
            month,
            rows.len()
        );
    }

    // Build a map for quick lookup
    let mut daily_map: HashMap<String, BalanceHistoryDailySummary> = HashMap::new();
    for row in rows {
        daily_map.insert(row.quota_day().format("%Y-%m-%d").to_string(), row);
    }

    // Without this, the first day of the month always reported a zero delta.
    let mut prev_consumed: Option<f64> = match first_day.pred_opt() {
        Some(previous_day) => balance_history_repo
            .find_daily_summary(&account, previous_day)
            .await?
            .map(|summary| summary.daily_consumed()),
        None => None,
    };

    let mut days = Vec::new();
    let mut checked_in_days = 0u32;
    let mut total_quota_increment = 0.0;

    for day in 1..=total_days {
        let date = NaiveDate::from_ymd_opt(year, month, day)
            .ok_or_else(|| DomainError::Validation("Invalid date".to_string()))?;
        let date_str = date.format("%Y-%m-%d").to_string();

        if let Some(row) = daily_map.get(&date_str) {
            let income_increment = row.check_in_reward();

            let daily_consumed = prev_consumed.map_or(0.0, |prev| {
                let diff = row.daily_consumed() - prev;
                if diff > 0.0 {
                    diff
                } else {
                    0.0
                }
            });

            let is_checked_in = row.is_checked_in();

            if is_checked_in {
                checked_in_days += 1;
                if let Some(inc) = income_increment {
                    total_quota_increment += inc;
                }
            }

            days.push(CheckInDayDto {
                date: date_str,
                is_checked_in,
                income_increment,
                reward_is_estimate: row.reward_is_estimate(),
                current_balance: row.daily_balance(),
                daily_consumed,
                total_consumed: row.daily_consumed(),
                total_quota: row.daily_total_quota(),
            });

            prev_consumed = Some(row.daily_consumed());
        } else {
            // No data for this day
            days.push(CheckInDayDto {
                date: date_str,
                is_checked_in: false,
                income_increment: None,
                reward_is_estimate: false,
                current_balance: 0.0,
                daily_consumed: 0.0,
                total_consumed: 0.0,
                total_quota: 0.0,
            });
        }
    }

    // The rate is measured against the days that have already started: a past
    // month has all of them, a future month none.
    let current_month = (today.year(), today.month());
    let viewed_month = (year, month);
    let elapsed_days = match viewed_month.cmp(&current_month) {
        std::cmp::Ordering::Less => total_days,
        std::cmp::Ordering::Greater => 0,
        std::cmp::Ordering::Equal => today.day().min(total_days),
    };

    let check_in_rate = if elapsed_days > 0 {
        (checked_in_days as f64 / elapsed_days as f64) * 100.0
    } else {
        0.0
    };

    let month_stats = MonthStatsDto {
        total_days,
        elapsed_days,
        checked_in_days,
        check_in_rate,
        total_quota_increment,
    };

    let dto = CheckInCalendarDto {
        account_id: account_id.to_string(),
        year,
        month,
        days,
        month_stats,
    };

    info!(
        "[streak] calendar result account_id={} checked_days={} elapsed_days={} rate={:.2}% income_delta={:.2}",
        dto.account_id,
        dto.month_stats.checked_in_days,
        dto.month_stats.elapsed_days,
        dto.month_stats.check_in_rate,
        dto.month_stats.total_quota_increment
    );

    Ok(dto)
}
