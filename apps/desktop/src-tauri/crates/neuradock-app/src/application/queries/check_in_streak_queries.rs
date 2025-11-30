use chrono::{Datelike, NaiveDate, Utc};
use log::{info, warn};
use sqlx::{FromRow, SqlitePool};
use std::collections::HashMap;
use std::sync::Arc;

use crate::application::dtos::{
    CheckInCalendarDto, CheckInDayDto, CheckInStreakDto, CheckInTrendDto, MonthStatsDto,
    TrendDataPoint,
};
use neuradock_domain::shared::DomainError;

#[derive(FromRow)]
struct DailySummaryRow {
    check_in_date: String,
    daily_total_income: f64,
    daily_balance: f64,
    daily_consumed: f64,
}

#[derive(FromRow)]
struct AccountInfoRow {
    name: String,
    provider_id: String,
    provider_name: String,
}

#[derive(FromRow)]
struct AccountListRow {
    account_id: String,
    name: String,
    provider_id: String,
    provider_name: String,
}

struct StreakComputation {
    current_streak: u32,
    longest_streak: u32,
    total_check_in_days: u32,
    last_check_in_date: Option<NaiveDate>,
}

pub struct CheckInStreakQueries {
    db: Arc<SqlitePool>,
}

impl CheckInStreakQueries {
    pub fn new(db: Arc<SqlitePool>) -> Self {
        Self { db }
    }

    /// Get streak statistics for a single account
    pub async fn get_streak_stats(
        &self,
        account_id: &str,
    ) -> Result<CheckInStreakDto, DomainError> {
        // Get account metadata and calculate streak from raw data
        let account_info = self.get_account_info(account_id).await?;
        let daily_rows = self.fetch_all_daily_summaries(account_id).await?;
        let streak = self.calculate_streak_stats(account_id, &daily_rows);

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
    pub async fn get_all_streaks(&self) -> Result<Vec<CheckInStreakDto>, DomainError> {
        let accounts = self.get_all_account_infos().await?;
        let mut results = Vec::new();

        for account in accounts {
            let daily_rows = self.fetch_all_daily_summaries(&account.account_id).await?;
            let streak = self.calculate_streak_stats(&account.account_id, &daily_rows);

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

        results.sort_by(|a, b| b.current_streak.cmp(&a.current_streak));

        info!("[streak] get_all_streaks total_accounts={}", results.len());
        Ok(results)
    }

    /// Get check-in calendar for a specific month
    pub async fn get_calendar(
        &self,
        account_id: &str,
        year: i32,
        month: u32,
    ) -> Result<CheckInCalendarDto, DomainError> {
        // Validate inputs
        if month < 1 || month > 12 {
            return Err(DomainError::Validation("Invalid month".to_string()));
        }

        // Calculate the first and last day of the month
        let first_day = NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| DomainError::Validation("Invalid date".to_string()))?;

        let last_day = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
                .unwrap()
                .pred_opt()
                .unwrap()
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
                .unwrap()
                .pred_opt()
                .unwrap()
        };

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
            .fetch_all(&*self.db)
            .await
            .map_err(|e| DomainError::Repository(e.to_string()))?;

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
            let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
            let date_str = date.format("%Y-%m-%d").to_string();

            if let Some(row) = daily_map.get(&date_str) {
                let income_increment = prev_income
                    .map(|prev| {
                        let diff = row.daily_total_income - prev;
                        if diff > 0.0 {
                            Some(diff)
                        } else {
                            None
                        }
                    })
                    .flatten();

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

    /// Get check-in trend data (last N days)
    pub async fn get_trend(
        &self,
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
            .fetch_all(&*self.db)
            .await
            .map_err(|e| DomainError::Repository(e.to_string()))?;

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
        &self,
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
            .fetch_optional(&*self.db)
            .await
            .map_err(|e| DomainError::Repository(e.to_string()))?;

        if let Some(row) = row {
            // Get previous day's income to calculate increment
            let prev_date = parsed_date.pred_opt().unwrap();
            let prev_query = r#"
                SELECT MAX(total_income)
                FROM balance_history
                WHERE account_id = ?1 AND DATE(recorded_at) = ?2
            "#;

            let prev_income: Option<f64> = sqlx::query_scalar(prev_query)
                .bind(account_id)
                .bind(prev_date.format("%Y-%m-%d").to_string())
                .fetch_optional(&*self.db)
                .await
                .map_err(|e| DomainError::Repository(e.to_string()))?;

            let income_increment = prev_income
                .map(|prev| {
                    let diff = row.daily_total_income - prev;
                    if diff > 0.0 {
                        Some(diff)
                    } else {
                        None
                    }
                })
                .flatten();

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

    /// Recalculate all streaks from balance_history
    pub async fn recalculate_all_streaks(&self) -> Result<(), DomainError> {
        let accounts_query = "SELECT DISTINCT account_id FROM balance_history";
        let account_ids: Vec<String> = sqlx::query_scalar(accounts_query)
            .fetch_all(&*self.db)
            .await
            .map_err(|e| DomainError::Repository(e.to_string()))?;

        info!(
            "[streak] recalc_all_streaks requested accounts={} (derived on demand)",
            account_ids.len()
        );

        Ok(())
    }

    // Helper: Get account metadata (name + provider)
    async fn get_account_info(&self, account_id: &str) -> Result<AccountInfoRow, DomainError> {
        let query = r#"
            SELECT
                a.name as name,
                a.provider_id as provider_id,
                COALESCE(p.name, a.provider_id) as provider_name
            FROM accounts a
            LEFT JOIN providers p ON p.id = a.provider_id
            WHERE a.id = ?1
        "#;

        let row: Option<AccountInfoRow> = sqlx::query_as(query)
            .bind(account_id)
            .fetch_optional(&*self.db)
            .await
            .map_err(|e| DomainError::Repository(e.to_string()))?;

        row.ok_or_else(|| DomainError::AccountNotFound(account_id.to_string()))
    }

    async fn get_all_account_infos(&self) -> Result<Vec<AccountListRow>, DomainError> {
        let query = r#"
            SELECT
                a.id as account_id,
                a.name as name,
                a.provider_id as provider_id,
                COALESCE(p.name, a.provider_id) as provider_name
            FROM accounts a
            LEFT JOIN providers p ON p.id = a.provider_id
            ORDER BY name ASC
        "#;

        sqlx::query_as(query)
            .fetch_all(&*self.db)
            .await
            .map_err(|e| DomainError::Repository(e.to_string()))
    }

    async fn fetch_all_daily_summaries(
        &self,
        account_id: &str,
    ) -> Result<Vec<DailySummaryRow>, DomainError> {
        let query = r#"
            SELECT
                DATE(recorded_at) AS check_in_date,
                MAX(total_income) AS daily_total_income,
                MAX(current_balance) AS daily_balance,
                MAX(total_consumed) AS daily_consumed
            FROM balance_history
            WHERE account_id = ?1
            GROUP BY DATE(recorded_at)
            ORDER BY check_in_date ASC
        "#;

        sqlx::query_as(query)
            .bind(account_id)
            .fetch_all(&*self.db)
            .await
            .map_err(|e| DomainError::Repository(e.to_string()))
    }

    fn calculate_streak_stats(
        &self,
        account_id: &str,
        rows: &[DailySummaryRow],
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
            let date = match NaiveDate::parse_from_str(&row.check_in_date, "%Y-%m-%d") {
                Ok(date) => date,
                Err(e) => {
                    warn!(
                        "[streak] skip invalid date row account_id={} value={} err={}",
                        account_id, row.check_in_date, e
                    );
                    continue;
                }
            };

            let is_checked_in = prev_income.map_or(true, |prev| row.daily_total_income > prev);

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

            prev_income = Some(row.daily_total_income);
        }

        StreakComputation {
            current_streak,
            longest_streak,
            total_check_in_days,
            last_check_in_date,
        }
    }
}
