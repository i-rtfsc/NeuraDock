//! Semantics of the calendar, trend, history and statistics read models.
//!
//! The streak arithmetic itself is unit tested in the domain
//! (`balance_history::streak`); these tests cover the parts that need a
//! database: the rate denominator, the first day's consumption baseline,
//! carrying a balance across days with no snapshot, pagination and how
//! statistics classify outcomes.

use chrono::{Datelike, Duration, NaiveDate, Utc};
use std::collections::HashMap;
use std::sync::Arc;

use neuradock_domain::account::AccountRepository;
use neuradock_domain::account::{Account, Credentials};
use neuradock_domain::balance_history::{
    AttemptCompletion, BalanceHistoryRecord, BalanceHistoryRepository, CheckInAttempt,
    CheckInOutcome, CheckInRecord, CheckInSource, CheckInTrigger,
};
use neuradock_domain::check_in::{today_quota_day, ProviderRepository};
use neuradock_domain::shared::{AccountId, ProviderId};
use neuradock_infrastructure::persistence::repositories::{
    SqliteAccountRepository, SqliteBalanceHistoryRepository, SqliteProviderRepository,
};
use neuradock_infrastructure::persistence::Database;
use neuradock_infrastructure::security::EncryptionService;

use super::CheckInStreakQueries;

const OFFSET_MINUTES: i32 = 480;

struct Fixture {
    _dir: tempfile::TempDir,
    account_id: AccountId,
    repo: Arc<SqliteBalanceHistoryRepository>,
    queries: CheckInStreakQueries,
}

impl Fixture {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let database = Database::new(dir.path().join("queries.db").to_str().unwrap())
            .await
            .unwrap();
        database.run_migrations().await.unwrap();
        let pool = Arc::new(database.pool().clone());

        let account_repo: Arc<dyn AccountRepository> = Arc::new(SqliteAccountRepository::new(
            pool.clone(),
            Arc::new(EncryptionService::from_key(&[3; 32]).unwrap()),
        ));
        let account = Account::new(
            "Queries".to_string(),
            ProviderId::from_string("test-provider"),
            Credentials::new(
                HashMap::from([("session".to_string(), "s".to_string())]),
                "u".to_string(),
            ),
        )
        .unwrap();
        account_repo.save(&account).await.unwrap();

        let provider_repo: Arc<dyn ProviderRepository> =
            Arc::new(SqliteProviderRepository::new(pool.clone()));
        let repo = Arc::new(SqliteBalanceHistoryRepository::new(pool.clone()));
        let queries = CheckInStreakQueries::new(account_repo, provider_repo, repo.clone());

        Self {
            _dir: dir,
            account_id: account.id().clone(),
            repo,
            queries,
        }
    }

    fn today(&self) -> NaiveDate {
        today_quota_day(OFFSET_MINUTES, Utc::now())
    }

    async fn snapshot(&self, day: NaiveDate, consumed: f64, quota: f64) {
        let recorded_at = day.and_hms_opt(12, 0, 0).unwrap().and_utc();
        self.repo
            .save(
                &BalanceHistoryRecord::new(
                    format!("{}-{day}", self.account_id.as_str()),
                    self.account_id.clone(),
                    quota - consumed,
                    consumed,
                    quota,
                    recorded_at,
                )
                .unwrap(),
            )
            .await
            .unwrap();
    }

    async fn evidence(&self, day: NaiveDate, reward: f64, source: CheckInSource) {
        let recorded_at = day.and_hms_opt(12, 0, 0).unwrap().and_utc();
        self.repo
            .save_check_in(
                &CheckInRecord::new(
                    self.account_id.clone(),
                    day,
                    recorded_at,
                    Some(reward),
                    source,
                )
                .unwrap(),
            )
            .await
            .unwrap();
    }

    /// Open an attempt on `day`, optionally closing it with an outcome.
    async fn attempt(
        &self,
        day: NaiveDate,
        outcome: Option<CheckInOutcome>,
        error_code: Option<&str>,
    ) -> i64 {
        let started_at = day.and_hms_opt(9, 0, 0).unwrap().and_utc();
        let attempt = CheckInAttempt::start(
            self.account_id.clone(),
            ProviderId::from_string("test-provider"),
            day,
            CheckInTrigger::Manual,
            started_at,
        );
        let id = self.repo.start_attempt(&attempt).await.unwrap();

        if let Some(outcome) = outcome {
            self.repo
                .finish_attempt(
                    id,
                    &AttemptCompletion {
                        outcome,
                        finished_at: started_at + Duration::seconds(4),
                        error_code: error_code.map(str::to_string),
                        http_status: None,
                        reward: None,
                        source: None,
                    },
                )
                .await
                .unwrap();
        }
        id
    }
}

#[tokio::test]
async fn the_rate_is_measured_against_days_that_have_started() {
    let fixture = Fixture::new().await;
    let today = fixture.today();
    fixture
        .evidence(today, 25.0, CheckInSource::ProviderResponse)
        .await;

    let calendar = fixture
        .queries
        .get_calendar(fixture.account_id.as_str(), today.year(), today.month())
        .await
        .unwrap();

    let stats = calendar.month_stats;
    assert_eq!(stats.checked_in_days, 1);
    assert_eq!(
        stats.elapsed_days,
        today.day(),
        "the current month has only the days that already started"
    );
    assert!((stats.check_in_rate - (100.0 / stats.elapsed_days as f64)).abs() < 1e-9);
    assert!(
        stats.elapsed_days <= stats.total_days,
        "the denominator can never exceed the month"
    );
}

#[tokio::test]
async fn a_past_month_counts_every_day() {
    let fixture = Fixture::new().await;
    let today = fixture.today();
    let (year, month) = if today.month() == 1 {
        (today.year() - 1, 12)
    } else {
        (today.year(), today.month() - 1)
    };

    let calendar = fixture
        .queries
        .get_calendar(fixture.account_id.as_str(), year, month)
        .await
        .unwrap();

    assert_eq!(
        calendar.month_stats.elapsed_days, calendar.month_stats.total_days,
        "a month that is over has all of its days elapsed"
    );
}

#[tokio::test]
async fn the_first_day_of_the_month_has_a_consumption_delta() {
    let fixture = Fixture::new().await;
    let today = fixture.today();
    let first_of_month = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
    let last_of_previous = first_of_month.pred_opt().unwrap();

    fixture.snapshot(last_of_previous, 10.0, 110.0).await;
    fixture.snapshot(first_of_month, 13.0, 113.0).await;

    let calendar = fixture
        .queries
        .get_calendar(fixture.account_id.as_str(), today.year(), today.month())
        .await
        .unwrap();

    let first_day = calendar.days.first().expect("day one exists");
    assert_eq!(
        first_day.date,
        first_of_month.format("%Y-%m-%d").to_string()
    );
    assert_eq!(
        first_day.daily_consumed, 3.0,
        "without the previous month as a baseline this was always zero"
    );
}

#[tokio::test]
async fn the_trend_carries_a_balance_across_days_without_a_snapshot() {
    let fixture = Fixture::new().await;
    let today = fixture.today();
    let three_days_ago = today - Duration::days(3);

    fixture.snapshot(three_days_ago, 0.0, 100.0).await;
    fixture.snapshot(today, 0.0, 125.0).await;
    fixture
        .evidence(today, 25.0, CheckInSource::ProviderResponse)
        .await;

    let trend = fixture
        .queries
        .get_trend(fixture.account_id.as_str(), 5)
        .await
        .unwrap();

    // Days before the first reading are omitted; the rest are filled in.
    assert_eq!(trend.data_points.len(), 4);
    assert_eq!(
        trend.data_points[0].date,
        three_days_ago.format("%Y-%m-%d").to_string()
    );

    let gap_day = &trend.data_points[1];
    assert_eq!(gap_day.total_quota, 100.0, "the last known reading carries");
    assert_eq!(gap_day.income_increment, None);
    assert!(!gap_day.is_checked_in);

    let last = trend.data_points.last().unwrap();
    assert_eq!(last.date, today.format("%Y-%m-%d").to_string());
    assert_eq!(last.income_increment, Some(25.0));
    assert!(last.is_checked_in);
}

#[tokio::test]
async fn the_calendar_rejects_an_impossible_month() {
    let fixture = Fixture::new().await;
    assert!(fixture
        .queries
        .get_calendar(fixture.account_id.as_str(), 2026, 13)
        .await
        .is_err());
}

#[tokio::test]
async fn the_day_detail_lookup_works_for_a_day_without_data() {
    let fixture = Fixture::new().await;
    let today = fixture.today();
    let missing = today - Duration::days(2);

    let detail = fixture
        .queries
        .get_day_detail(
            fixture.account_id.as_str(),
            &missing.format("%Y-%m-%d").to_string(),
        )
        .await
        .unwrap();

    assert!(!detail.is_checked_in);
    assert_eq!(detail.income_increment, None);
}

#[tokio::test]
async fn history_pages_attempts_newest_first() {
    let fixture = Fixture::new().await;
    let today = fixture.today();
    let yesterday = today.pred_opt().unwrap();
    let day_before = yesterday.pred_opt().unwrap();

    fixture
        .attempt(day_before, Some(CheckInOutcome::Success), None)
        .await;
    fixture
        .attempt(
            yesterday,
            Some(CheckInOutcome::FailedRetryable),
            Some("network"),
        )
        .await;
    fixture.attempt(today, None, None).await;

    let first_page = fixture
        .queries
        .get_history(Some(fixture.account_id.as_str()), 1, 2)
        .await
        .unwrap();
    assert_eq!(first_page.total, 3);
    assert_eq!(first_page.attempts.len(), 2);
    assert_eq!(
        first_page.attempts[0].quota_day,
        today.format("%Y-%m-%d").to_string(),
        "newest first"
    );
    assert_eq!(first_page.attempts[0].outcome, None, "an open attempt");
    assert_eq!(
        first_page.attempts[1].error_code.as_deref(),
        Some("network")
    );

    let second_page = fixture
        .queries
        .get_history(Some(fixture.account_id.as_str()), 2, 2)
        .await
        .unwrap();
    assert_eq!(second_page.attempts.len(), 1);
    assert_eq!(
        second_page.attempts[0].quota_day,
        day_before.format("%Y-%m-%d").to_string()
    );

    // All accounts share the same page when no filter is given.
    let unfiltered = fixture.queries.get_history(None, 1, 10).await.unwrap();
    assert_eq!(unfiltered.total, 3);
}

#[tokio::test]
async fn statistics_separate_failures_from_skips_and_count_evidence() {
    let fixture = Fixture::new().await;
    let today = fixture.today();
    let yesterday = today.pred_opt().unwrap();
    let day_before = yesterday.pred_opt().unwrap();

    fixture
        .attempt(day_before, Some(CheckInOutcome::Success), None)
        .await;
    fixture
        .attempt(
            yesterday,
            Some(CheckInOutcome::FailedRetryable),
            Some("network"),
        )
        .await;
    fixture
        .attempt(today, Some(CheckInOutcome::SkippedGuard), None)
        .await;

    // One confirmed day and one inferred day in the period.
    fixture
        .evidence(day_before, 25.0, CheckInSource::ProviderResponse)
        .await;
    fixture
        .evidence(yesterday, 3.0, CheckInSource::BalanceDelta)
        .await;

    let stats = fixture
        .queries
        .get_stats(fixture.account_id.as_str(), 30)
        .await
        .unwrap();

    assert_eq!(stats.attempts, 3);
    assert_eq!(stats.succeeded, 1);
    assert_eq!(stats.failed, 1);
    assert_eq!(stats.skipped, 1, "a guard skip is not a failure");
    assert_eq!(
        stats.success_rate, 50.0,
        "the rate is over resolved attempts, so skips do not dilute it"
    );
    assert_eq!(stats.average_duration_ms, Some(4_000));
    assert_eq!(stats.confirmed_days, 1);
    assert_eq!(stats.inferred_days, 1);
    assert_eq!(stats.total_reward, 28.0);
    assert_eq!(stats.failures_by_code.len(), 1);
    assert_eq!(stats.failures_by_code[0].error_code, "network");
    assert_eq!(stats.failures_by_code[0].count, 1);
}
