use chrono::{Datelike, NaiveDate};
use log::{info, warn};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;

use crate::application::dtos::{CheckInCalendarDto, CheckInDayDto, MonthStatsDto};
use crate::application::ResultExt;
use neuradock_domain::shared::DomainError;

use super::types::DailySummaryRow;

/// Get check-in calendar for a specific month
pub async fn get_calendar(
    db: &Arc<SqlitePool>,
    account_id: &str,
    year: i32,
    month: u32,
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

    // Query daily summaries for this month
    let query = r#"
        WITH daily_summary AS (
            SELECT
                DATE(recorded_at) AS check_in_date,
                MAX(total_income) AS daily_total_income,
                MAX(current_balance) AS daily_balance,
                MAX(total_consumed) AS daily_consumed
            FROM balance_history
            WHERE account_id = ?1
            GROUP BY DATE(recorded_at)
        )
        SELECT
            check_in_date,
            daily_total_income,
            daily_balance,
            daily_consumed
        FROM daily_summary
        WHERE check_in_date >= ?2
          AND check_in_date <= ?3
        ORDER BY check_in_date ASC
    "#;

    let rows: Vec<DailySummaryRow> = sqlx::query_as(query)
        .bind(account_id)
        .bind(first_day.format("%Y-%m-%d").to_string())
        .bind(last_day.format("%Y-%m-%d").to_string())
        .fetch_all(&**db)
        .await
        .to_repo_err()?;

    if rows.is_empty() {
        warn!(
            "[streak] calendar query empty result account_id={} month={}",
            account_id,
            format!("{:04}-{:02}", year, month)
        );
    } else {
        info!(
            "[streak] calendar query account_id={} month={} rows={}",
            account_id,
            format!("{:04}-{:02}", year, month),
            rows.len()
        );
    }

    // Build a map for quick lookup
    let mut daily_map: HashMap<String, DailySummaryRow> = HashMap::new();
    for row in rows {
        daily_map.insert(row.check_in_date.clone(), row);
    }

    // Calculate income increments
    let mut days = Vec::new();
    let mut prev_income: Option<f64> = None;
    let mut checked_in_days = 0u32;
    let mut total_income_increment = 0.0;

    let total_days = last_day.day();

    for day in 1..=total_days {
        let date = NaiveDate::from_ymd_opt(year, month, day)
            .ok_or_else(|| DomainError::Validation("Invalid date".to_string()))?;
        let date_str = date.format("%Y-%m-%d").to_string();

        if let Some(row) = daily_map.get(&date_str) {
            let income_increment = prev_income
                .and_then(|prev| {
                    let diff = row.daily_total_income - prev;
                    if diff > 0.0 {
                        Some(diff)
                    } else {
                        None
                    }
                });

            let is_checked_in = income_increment.is_some() || prev_income.is_none();

            if is_checked_in {
                checked_in_days += 1;
                if let Some(inc) = income_increment {
                    total_income_increment += inc;
                } else if prev_income.is_none() && row.daily_total_income > 0.0 {
                    // First record, count as income
                    total_income_increment += row.daily_total_income;
                }
            }

            days.push(CheckInDayDto {
                date: date_str,
                is_checked_in,
                income_increment,
                current_balance: row.daily_balance,
                total_consumed: row.daily_consumed,
                total_income: row.daily_total_income,
            });

            prev_income = Some(row.daily_total_income);
        } else {
            // No data for this day
            days.push(CheckInDayDto {
                date: date_str,
                is_checked_in: false,
                income_increment: None,
                current_balance: 0.0,
                total_consumed: 0.0,
                total_income: 0.0,
            });
        }
    }

    let check_in_rate = if total_days > 0 {
        (checked_in_days as f64 / total_days as f64) * 100.0
    } else {
        0.0
    };

    let month_stats = MonthStatsDto {
        total_days,
        checked_in_days,
        check_in_rate,
        total_income_increment,
    };

    let dto = CheckInCalendarDto {
        account_id: account_id.to_string(),
        year,
        month,
        days,
        month_stats,
    };

    info!(
        "[streak] calendar result account_id={} checked_days={} rate={:.2}% income_delta={:.2}",
        dto.account_id,
        dto.month_stats.checked_in_days,
        dto.month_stats.check_in_rate,
        dto.month_stats.total_income_increment
    );

    Ok(dto)
}
