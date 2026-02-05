use chrono::{NaiveDate, Utc};
use log::{info, warn};

use crate::application::dtos::{CheckInDayDto, CheckInTrendDto, TrendDataPoint};
use neuradock_domain::balance_history::BalanceHistoryRepository;
use neuradock_domain::shared::AccountId;
use neuradock_domain::shared::DomainError;

/// Get check-in trend data (last N days)
pub async fn get_trend(
    balance_history_repo: &dyn BalanceHistoryRepository,
    account_id: &str,
    days: u32,
) -> Result<CheckInTrendDto, DomainError> {
    if days == 0 || days > 365 {
        return Err(DomainError::Validation(
            "Days must be between 1 and 365".to_string(),
        ));
    }

    let end_date = Utc::now().naive_utc().date();
    let start_date = end_date - chrono::Duration::days(days as i64 - 1);

    let rows = balance_history_repo
        .list_daily_summaries_in_range(&AccountId::from_string(account_id), start_date, end_date)
        .await?;

    info!(
        "[streak] trend query account_id={} range={}~{} rows={}",
        account_id,
        start_date,
        end_date,
        rows.len()
    );

    let mut data_points = Vec::new();
    let mut prev_income: Option<f64> = None;

    for row in rows {
        let income_increment = if let Some(prev) = prev_income {
            let diff = row.daily_total_quota() - prev;
            if diff > 0.0 { diff } else { 0.0 }
        } else {
            row.daily_total_quota() // First day handle
        };

        let is_checked_in = income_increment > 0.0 || prev_income.is_none();

        data_points.push(TrendDataPoint {
            date: row.check_in_date().format("%Y-%m-%d").to_string(),
            total_quota: row.daily_total_quota(),
            income_increment,
            current_balance: row.daily_balance(),
            is_checked_in,
        });

        prev_income = Some(row.daily_total_quota());
    }

    let dto = CheckInTrendDto {
        account_id: account_id.to_string(),
        start_date: start_date.format("%Y-%m-%d").to_string(),
        end_date: end_date.format("%Y-%m-%d").to_string(),
        data_points,
    };

    if dto.data_points.is_empty() {
        warn!("[streak] trend result empty account_id={}", dto.account_id);
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

        let income_increment = prev_summary.as_ref().and_then(|prev| {
            let diff = row.daily_total_quota() - prev.daily_total_quota();
            if diff > 0.0 {
                Some(diff)
            } else {
                None
            }
        });

        let daily_consumed = prev_summary.as_ref().map_or(0.0, |prev| {
            let diff = row.daily_consumed() - prev.daily_consumed();
            if diff > 0.0 {
                diff
            } else {
                0.0
            }
        });

        let is_checked_in = income_increment.is_some() || prev_summary.is_none();

        Ok(CheckInDayDto {
            date: date.to_string(),
            is_checked_in,
            income_increment,
            current_balance: row.daily_balance(),
            daily_consumed,
            total_consumed: row.daily_consumed(),
            total_quota: row.daily_total_quota(),
        })
    } else {
        warn!(
            "[streak] day detail not found account_id={} date={}",
            account_id, date
        );
        // No data for this day
        Ok(CheckInDayDto {
            date: date.to_string(),
            is_checked_in: false,
            income_increment: None,
            current_balance: 0.0,
            daily_consumed: 0.0,
            total_consumed: 0.0,
            total_quota: 0.0,
        })
    }
}
