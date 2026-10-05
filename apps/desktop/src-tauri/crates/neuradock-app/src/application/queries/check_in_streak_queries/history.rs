use chrono::{Duration, Utc};
use std::collections::HashMap;

use crate::application::dtos::{
    CheckInAttemptDto, CheckInFailureDto, CheckInHistoryPageDto, CheckInStatsDto,
};
use neuradock_domain::balance_history::{
    CheckInAttempt, CheckInConfidence, CheckInOutcome, CheckInSource,
};
use neuradock_domain::check_in::today_quota_day;
use neuradock_domain::shared::{AccountId, DomainError};

use super::helpers;
use super::CheckInStreakQueries;

const MAX_PAGE_SIZE: i32 = 200;
const MAX_PERIOD_DAYS: u32 = 365;
const UNKNOWN_FAILURE: &str = "unknown";

impl CheckInStreakQueries {
    /// Paginated attempt history, newest first.
    ///
    /// `account_id` narrows it to one account; `None` lists every account.
    pub async fn get_history(
        &self,
        account_id: Option<&str>,
        page: i32,
        page_size: i32,
    ) -> Result<CheckInHistoryPageDto, DomainError> {
        let page = page.max(1);
        let page_size = page_size.clamp(1, MAX_PAGE_SIZE);
        let filter = account_id.map(AccountId::from_string);

        let total = self
            .balance_history_repo
            .count_attempts(filter.as_ref())
            .await?;
        let attempts = self
            .balance_history_repo
            .list_attempts(
                filter.as_ref(),
                i64::from(page_size),
                i64::from((page - 1) * page_size),
            )
            .await?;

        let names = self.name_lookup().await?;
        let dtos = attempts
            .iter()
            .map(|attempt| {
                let (account_name, provider_name) = names
                    .get(attempt.account_id().as_str())
                    .cloned()
                    .unwrap_or_else(|| (attempt.account_id().as_str().to_string(), String::new()));
                attempt_to_dto(attempt, account_name, provider_name)
            })
            .collect();

        Ok(CheckInHistoryPageDto {
            total,
            page,
            page_size,
            attempts: dtos,
        })
    }

    /// Attempt and evidence statistics for one account over the last N days.
    pub async fn get_stats(
        &self,
        account_id: &str,
        period_days: u32,
    ) -> Result<CheckInStatsDto, DomainError> {
        let period_days = period_days.clamp(1, MAX_PERIOD_DAYS);
        let info = helpers::get_account_info(
            self.account_repo.as_ref(),
            self.provider_repo.as_ref(),
            account_id,
        )
        .await?;

        let end_date = today_quota_day(info.provider_day_offset_minutes, Utc::now());
        let start_date = end_date - Duration::days(i64::from(period_days) - 1);
        let id = AccountId::from_string(account_id);

        let attempts = self
            .balance_history_repo
            .list_attempts_in_range(&id, start_date, end_date)
            .await?;
        let evidence = self
            .balance_history_repo
            .list_evidence_in_range(&id, start_date, end_date)
            .await?;

        let mut succeeded = 0i32;
        let mut failed = 0i32;
        let mut skipped = 0i32;
        let mut durations: Vec<i64> = Vec::new();
        let mut failures: HashMap<String, i32> = HashMap::new();

        for attempt in &attempts {
            match attempt.outcome() {
                Some(CheckInOutcome::Success) => succeeded += 1,
                Some(outcome) if outcome.is_failure() => {
                    failed += 1;
                    let code = attempt.error_code().unwrap_or(UNKNOWN_FAILURE).to_string();
                    *failures.entry(code).or_insert(0) += 1;
                }
                _ => skipped += 1,
            }
            if let Some(duration) = attempt.duration_ms() {
                durations.push(duration);
            }
        }

        let resolved = succeeded + failed;
        let success_rate = if resolved > 0 {
            f64::from(succeeded) / f64::from(resolved) * 100.0
        } else {
            0.0
        };

        let average_duration_ms = if durations.is_empty() {
            None
        } else {
            Some(durations.iter().sum::<i64>() / durations.len() as i64)
        };

        let confirmed_days = evidence
            .iter()
            .filter(|record| record.confidence() != CheckInConfidence::Inferred)
            .count() as i32;
        let inferred_days = evidence
            .iter()
            .filter(|record| record.confidence() == CheckInConfidence::Inferred)
            .count() as i32;
        let total_reward = evidence
            .iter()
            .filter_map(|record| record.reward())
            .sum::<f64>();

        let mut failures_by_code: Vec<CheckInFailureDto> = failures
            .into_iter()
            .map(|(error_code, count)| CheckInFailureDto { error_code, count })
            .collect();
        failures_by_code
            .sort_by(|a, b| b.count.cmp(&a.count).then(a.error_code.cmp(&b.error_code)));

        Ok(CheckInStatsDto {
            account_id: account_id.to_string(),
            period_days,
            start_date: start_date.format("%Y-%m-%d").to_string(),
            end_date: end_date.format("%Y-%m-%d").to_string(),
            attempts: attempts.len() as i32,
            succeeded,
            failed,
            skipped,
            success_rate,
            average_duration_ms,
            confirmed_days,
            inferred_days,
            total_reward,
            failures_by_code,
        })
    }

    /// Account and provider display names, keyed by account id.
    async fn name_lookup(&self) -> Result<HashMap<String, (String, String)>, DomainError> {
        let accounts = self.account_repo.find_all().await?;
        let providers = self.provider_repo.find_all().await?;
        let provider_names: HashMap<String, String> = providers
            .into_iter()
            .map(|provider| {
                (
                    provider.id().as_str().to_string(),
                    provider.name().to_string(),
                )
            })
            .collect();

        Ok(accounts
            .into_iter()
            .map(|account| {
                let provider_name = provider_names
                    .get(account.provider_id().as_str())
                    .cloned()
                    .unwrap_or_default();
                (
                    account.id().as_str().to_string(),
                    (account.name().to_string(), provider_name),
                )
            })
            .collect())
    }
}

fn attempt_to_dto(
    attempt: &CheckInAttempt,
    account_name: String,
    provider_name: String,
) -> CheckInAttemptDto {
    CheckInAttemptDto {
        id: attempt.id().unwrap_or_default(),
        account_id: attempt.account_id().as_str().to_string(),
        account_name,
        provider_name,
        quota_day: attempt.quota_day().format("%Y-%m-%d").to_string(),
        trigger: attempt.trigger().as_str().to_string(),
        outcome: attempt
            .outcome()
            .map(|outcome| outcome.as_str().to_string()),
        error_code: attempt.error_code().map(str::to_string),
        http_status: attempt.http_status(),
        started_at: attempt.started_at().to_rfc3339(),
        finished_at: attempt.finished_at().map(|at| at.to_rfc3339()),
        duration_ms: attempt.duration_ms(),
        reward: attempt.reward_cents().map(|cents| cents as f64 / 100.0),
        source: attempt
            .source()
            .map(|source: CheckInSource| source.as_str().to_string()),
    }
}
