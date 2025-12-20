use chrono::{NaiveDate, Utc};
use log::{info, warn};
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::application::dtos::{CheckInDayDto, CheckInTrendDto, TrendDataPoint};
use crate::application::ResultExt;
use neuradock_domain::shared::DomainError;

use super::types::DailySummaryRow;

/// Get check-in trend data (last N days)
pub async fn get_trend(
    db: &Arc<SqlitePool>,
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
        .bind(start_date.format("%Y-%m-%d").to_string())
        .bind(end_date.format("%Y-%m-%d").to_string())
        .fetch_all(&**db)
        .await
        .to_repo_err()?;

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
        let income_increment = prev_income.map_or(0.0, |prev| {
            let diff = row.daily_total_income - prev;
            if diff > 0.0 {
                diff
            } else {
                0.0
            }
        });

        let is_checked_in = income_increment > 0.0 || prev_income.is_none();

        data_points.push(TrendDataPoint {
            date: row.check_in_date,
            total_income: row.daily_total_income,
            income_increment,
            current_balance: row.daily_balance,
            is_checked_in,
        });

        prev_income = Some(row.daily_total_income);
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
    db: &Arc<SqlitePool>,
    account_id: &str,
    date: &str,
) -> Result<CheckInDayDto, DomainError> {
    // Validate date format
    let parsed_date = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| {
        DomainError::Validation("Invalid date format, expected YYYY-MM-DD".to_string())
    })?;

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
        WHERE check_in_date = ?2
    "#;

    let row: Option<DailySummaryRow> = sqlx::query_as(query)
        .bind(account_id)
        .bind(date)
        .fetch_optional(&**db)
        .await
        .to_repo_err()?;

    if let Some(row) = row {
        // Get previous day's income to calculate increment
        let prev_income: Option<f64> = if let Some(prev_date) = parsed_date.pred_opt() {
            let prev_query = r#"
                SELECT MAX(total_income)
                FROM balance_history
                WHERE account_id = ?1 AND DATE(recorded_at) = ?2
            "#;
            sqlx::query_scalar(prev_query)
                .bind(account_id)
                .bind(prev_date.format("%Y-%m-%d").to_string())
                .fetch_optional(&**db)
                .await
                .to_repo_err()?
        } else {
            None
        };

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

        Ok(CheckInDayDto {
            date: date.to_string(),
            is_checked_in,
            income_increment,
            current_balance: row.daily_balance,
            total_consumed: row.daily_consumed,
            total_income: row.daily_total_income,
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
            total_consumed: 0.0,
            total_income: 0.0,
        })
    }
}
