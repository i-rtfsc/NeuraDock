use chrono::{DateTime, Duration, NaiveDate, Utc};
use neuradock_domain::balance_history::{
    AttemptCompletion, BalanceHistoryRecord, BalanceHistoryRepository, CheckInAttempt,
    CheckInOutcome, CheckInRecord, CheckInSource, CheckInTrigger,
};
use neuradock_domain::shared::AccountId;
use neuradock_infrastructure::persistence::repositories::SqliteBalanceHistoryRepository;
use sqlx::SqlitePool;
use std::sync::Arc;

mod test_helpers;

/// The fixture account has no `providers` row, so every query falls back to the
/// default UTC+8 quota day. Tests therefore only need to care about that, and
/// the offset itself is covered by the domain's quota_day tests.
const OFFSET_MINUTES: i32 = 480;

async fn fixture() -> (SqlitePool, SqliteBalanceHistoryRepository, AccountId) {
    let (pool, _) = test_helpers::setup_in_memory_db().await;
    let id = AccountId::new();
    sqlx::query("INSERT INTO accounts (id,name,provider_id,cookies,api_user,created_at) VALUES (?,'Test account','test-provider','test-cookie','test-user',datetime('now'))")
        .bind(id.as_str()).execute(&pool).await.unwrap();
    let repo = SqliteBalanceHistoryRepository::new(Arc::new(pool.clone()));
    (pool, repo, id)
}

fn day(value: &str) -> NaiveDate {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").expect("valid day")
}

fn at(value: &str) -> DateTime<Utc> {
    value.parse().expect("valid RFC3339 instant")
}

fn evidence(
    id: &AccountId,
    quota_day: &str,
    recorded_at: DateTime<Utc>,
    reward: Option<f64>,
    source: CheckInSource,
) -> CheckInRecord {
    CheckInRecord::new(id.clone(), day(quota_day), recorded_at, reward, source).unwrap()
}

fn snapshot(
    id: &AccountId,
    current: f64,
    consumed: f64,
    recorded_at: DateTime<Utc>,
) -> BalanceHistoryRecord {
    BalanceHistoryRecord::new(
        format!(
            "{}-{}",
            id.as_str(),
            recorded_at.timestamp_nanos_opt().unwrap()
        ),
        id.clone(),
        current,
        consumed,
        current + consumed,
        recorded_at,
    )
    .unwrap()
}

async fn cache(pool: &SqlitePool, id: &AccountId) -> (Option<String>, Option<String>) {
    sqlx::query_as::<_, (Option<String>, Option<String>)>(
        "SELECT last_confirmed_at, last_evidence_at FROM accounts WHERE id = ?",
    )
    .bind(id.as_str())
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn concurrent_usage_refresh_and_retries_cannot_change_a_confirmed_reward() {
    let (pool, repo, id) = fixture().await;
    let now = at("2026-01-02T12:00:00Z");
    let today = "2026-01-02";

    repo.save(&snapshot(&id, 100.0, 200.0, now - Duration::days(1)))
        .await
        .unwrap();
    let reward = evidence(&id, today, now, Some(25.0), CheckInSource::ProviderRule);
    assert!(repo.save_check_in(&reward).await.unwrap());

    // The agent's 1.56 pre-charge is visible before used_quota settles.
    let pending_usage = snapshot(&id, 123.44, 200.0, now);
    let (refresh, duplicate) = tokio::join!(repo.save(&pending_usage), repo.save_check_in(&reward));
    refresh.unwrap();
    assert!(
        !duplicate.unwrap(),
        "a retry must not rewrite a settled row"
    );

    let daily = repo
        .find_daily_summary(&id, day(today))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(daily.check_in_reward(), Some(25.0));
    assert!(daily.is_checked_in());
    assert!(!daily.reward_is_estimate());

    // Settlement, further consumption and a top-up only change balances.
    for (current, consumed) in [(123.44, 201.56), (50.0, 275.0), (1050.0, 275.0)] {
        repo.save(&snapshot(
            &id,
            current,
            consumed,
            now + Duration::seconds(1),
        ))
        .await
        .unwrap();
        let daily = repo
            .find_daily_summary(&id, day(today))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(daily.check_in_reward(), Some(25.0));
        assert_eq!(daily.daily_balance(), current);
    }

    let unknown = evidence(&id, today, now, None, CheckInSource::ConfirmedUnknown);
    assert!(
        !repo.save_check_in(&unknown).await.unwrap(),
        "a confirmed-unknown row must not overwrite a settled reward"
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM check_in_records WHERE account_id=?")
        .bind(id.as_str())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn balance_changes_alone_do_not_create_check_ins() {
    let (_, repo, id) = fixture().await;
    let now = at("2026-01-02T12:00:00Z");

    repo.save(&snapshot(&id, 1000.0, 200.0, now)).await.unwrap();

    let daily = repo
        .find_daily_summary(&id, day("2026-01-02"))
        .await
        .unwrap()
        .unwrap();
    assert!(!daily.is_checked_in());
    assert_eq!(daily.check_in_reward(), None);
}

#[tokio::test]
async fn rewards_survive_missing_balance_responses_and_unknown_amounts_stay_unknown() {
    let (_, repo, id) = fixture().await;
    let first = at("2026-01-02T12:00:00Z");

    repo.save_check_in(&evidence(
        &id,
        "2026-01-02",
        first,
        Some(25.0),
        CheckInSource::ProviderResponse,
    ))
    .await
    .unwrap();
    repo.save_check_in(&evidence(
        &id,
        "2026-01-03",
        first + Duration::days(1),
        None,
        CheckInSource::ConfirmedUnknown,
    ))
    .await
    .unwrap();

    let rows = repo.list_all_daily_summaries(&id).await.unwrap();
    assert_eq!(rows.len(), 2, "evidence alone still produces day rows");
    assert_eq!(rows[0].check_in_reward(), Some(25.0));
    assert!(rows[1].is_checked_in());
    assert_eq!(rows[1].check_in_reward(), None);
    assert!(!rows[1].reward_is_estimate());
}

/// The ladder from ADR-007 D-5: only promotion, never demotion, and a missing
/// amount can always be filled in.
#[tokio::test]
async fn the_evidence_ladder_only_promotes() {
    let (_, repo, id) = fixture().await;
    let now = at("2026-01-02T12:00:00Z");
    let today = day("2026-01-02");

    // A legacy estimate starts the day.
    assert!(repo
        .save_check_in(&evidence(
            &id,
            "2026-01-02",
            now,
            None,
            CheckInSource::LegacyEstimate
        ))
        .await
        .unwrap());

    // An inferred balance delta is more trustworthy than a legacy estimate.
    assert!(repo
        .save_check_in(&evidence(
            &id,
            "2026-01-02",
            now,
            Some(3.0),
            CheckInSource::BalanceDelta
        ))
        .await
        .unwrap());
    // A provider rule beats that.
    assert!(repo
        .save_check_in(&evidence(
            &id,
            "2026-01-02",
            now,
            Some(25.0),
            CheckInSource::ProviderRule
        ))
        .await
        .unwrap());
    // The provider's own response beats the rule.
    assert!(repo
        .save_check_in(&evidence(
            &id,
            "2026-01-02",
            now,
            Some(24.0),
            CheckInSource::ProviderResponse
        ))
        .await
        .unwrap());
    // A manual correction beats everything (ADR-007 U1).
    assert!(repo
        .save_check_in(&evidence(
            &id,
            "2026-01-02",
            now,
            Some(30.0),
            CheckInSource::Manual
        ))
        .await
        .unwrap());

    let row = repo.find_evidence(&id, today).await.unwrap().unwrap();
    assert_eq!(row.source(), CheckInSource::Manual);
    assert_eq!(row.reward(), Some(30.0));
    assert_eq!(
        row.confidence(),
        neuradock_domain::balance_history::CheckInConfidence::Manual
    );

    // Nothing demotes a manual row.
    for source in [
        CheckInSource::ProviderResponse,
        CheckInSource::BalanceDelta,
        CheckInSource::LegacyEstimate,
    ] {
        assert!(!repo
            .save_check_in(&evidence(&id, "2026-01-02", now, Some(99.0), source))
            .await
            .unwrap());
    }
    let row = repo.find_evidence(&id, today).await.unwrap().unwrap();
    assert_eq!(row.source(), CheckInSource::Manual);
    assert_eq!(row.reward(), Some(30.0));
}

/// Invariant: a day with a confirmed-unknown reward can still learn the amount.
#[tokio::test]
async fn an_unknown_amount_is_filled_in_without_changing_the_day() {
    let (_, repo, id) = fixture().await;
    let now = at("2026-01-02T12:00:00Z");

    repo.save_check_in(&evidence(
        &id,
        "2026-01-02",
        now,
        None,
        CheckInSource::ConfirmedUnknown,
    ))
    .await
    .unwrap();

    assert!(repo
        .save_check_in(&evidence(
            &id,
            "2026-01-02",
            now,
            Some(25.0),
            CheckInSource::ProviderRule,
        ))
        .await
        .unwrap());

    let row = repo
        .find_evidence(&id, day("2026-01-02"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.reward(), Some(25.0));
    assert_eq!(row.source(), CheckInSource::ProviderRule);
}

#[tokio::test]
async fn the_account_cache_separates_confirmed_from_observed() {
    let (pool, repo, id) = fixture().await;

    // An inferred day is observed but not confirmed.
    repo.save_check_in(&evidence(
        &id,
        "2026-01-02",
        at("2026-01-02T12:00:00Z"),
        Some(3.0),
        CheckInSource::BalanceDelta,
    ))
    .await
    .unwrap();

    let (confirmed, observed) = cache(&pool, &id).await;
    assert_eq!(confirmed, None, "an inference is not a confirmation");
    assert_eq!(observed.as_deref(), Some("2026-01-02T12:00:00+00:00"));

    // A provider response on a later day confirms it.
    repo.save_check_in(&evidence(
        &id,
        "2026-01-03",
        at("2026-01-03T12:00:00Z"),
        Some(25.0),
        CheckInSource::ProviderResponse,
    ))
    .await
    .unwrap();

    let (confirmed, observed) = cache(&pool, &id).await;
    assert_eq!(confirmed.as_deref(), Some("2026-01-03T12:00:00+00:00"));
    assert_eq!(observed.as_deref(), Some("2026-01-03T12:00:00+00:00"));
}

/// Invariant: a snapshot and the evidence it implies land in one transaction.
///
/// Whether an observation *implies* evidence (a quota increase over the
/// previous quota day, and never on the first snapshot) is decided by
/// `BalanceHistoryService`; the repository stays a dumb writer, so that rule is
/// covered by the application-layer tests.
#[tokio::test]
async fn balance_observations_store_snapshots_with_or_without_evidence() {
    let (pool, repo, id) = fixture().await;
    let first = at("2026-01-02T12:00:00Z");
    let second = at("2026-01-03T12:00:00Z");

    // An observation that proves nothing stores only the snapshot.
    assert!(!repo
        .save_balance_observation(&snapshot(&id, 100.0, 0.0, first), None)
        .await
        .unwrap());
    assert!(repo
        .find_evidence(&id, day("2026-01-02"))
        .await
        .unwrap()
        .is_none());

    // An observation that implies evidence stores both.
    let inferred = evidence(
        &id,
        "2026-01-03",
        second,
        Some(25.0),
        CheckInSource::BalanceDelta,
    );
    assert!(repo
        .save_balance_observation(&snapshot(&id, 125.0, 0.0, second), Some(&inferred))
        .await
        .unwrap());

    let stored = repo
        .find_evidence(&id, day("2026-01-03"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.reward(), Some(25.0));
    assert!(stored.reward_is_estimate());

    // The snapshot and the evidence are both visible.
    let daily = repo
        .find_daily_summary(&id, day("2026-01-03"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(daily.daily_balance(), 125.0);
    assert!(daily.is_checked_in());

    // A later no-op observation keeps the cache pointing at the evidence.
    assert!(!repo
        .save_balance_observation(
            &snapshot(&id, 120.0, 5.0, second + Duration::hours(1)),
            None
        )
        .await
        .unwrap());
    let (_, observed) = cache(&pool, &id).await;
    assert_eq!(observed.as_deref(), Some("2026-01-03T12:00:00+00:00"));
}

#[tokio::test]
async fn open_attempts_are_leased_and_closed_with_an_outcome() {
    let (_, repo, id) = fixture().await;
    let now = at("2026-01-02T12:00:00Z");
    let today = day("2026-01-02");

    let attempt = CheckInAttempt::start(
        id.clone(),
        neuradock_domain::shared::ProviderId::from_string("test-provider"),
        today,
        CheckInTrigger::Scheduler,
        now,
    );
    let attempt_id = repo.start_attempt(&attempt).await.unwrap();

    // The lease only sees attempts started inside its window.
    let open = repo
        .find_open_attempt(&id, today, now - Duration::minutes(10))
        .await
        .unwrap();
    assert_eq!(open.map(|attempt| attempt.id()), Some(Some(attempt_id)));
    assert!(repo
        .find_open_attempt(&id, today, now + Duration::minutes(1))
        .await
        .unwrap()
        .is_none());

    repo.finish_attempt(
        attempt_id,
        &AttemptCompletion {
            outcome: CheckInOutcome::FailedRetryable,
            finished_at: now + Duration::seconds(30),
            error_code: Some("network".to_string()),
            http_status: None,
            reward: None,
            source: None,
        },
    )
    .await
    .unwrap();

    let closed = repo
        .find_open_attempt(&id, today, now - Duration::minutes(10))
        .await
        .unwrap();
    assert!(closed.is_none(), "a finished attempt is not in flight");

    let attempts = repo.list_attempts_on_day(&id, today).await.unwrap();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].outcome(), Some(CheckInOutcome::FailedRetryable));
    assert_eq!(attempts[0].error_code(), Some("network"));
    assert_eq!(attempts[0].duration_ms(), Some(30_000));
}

/// The rebuild migration re-buckets history by the provider offset and keeps
/// exactly one row per quota day, choosing by the same ladder the runtime uses.
#[tokio::test]
async fn the_rebuild_migration_rebuckets_legacy_evidence() {
    let (pool, repo, id) = fixture().await;

    // Recreate the legacy shape, as it existed before the rebuild.
    sqlx::query("DROP TABLE check_in_records")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(
        r#"
        CREATE TABLE check_in_records (
            account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
            check_in_date TEXT NOT NULL,
            reward_cents INTEGER CHECK (reward_cents IS NULL OR reward_cents >= 0),
            source TEXT NOT NULL CHECK (source IN ('provider_response','provider_rule','confirmed_unknown','legacy_estimate','manual')),
            recorded_at TEXT NOT NULL,
            PRIMARY KEY (account_id, check_in_date)
        );
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    // The first day has no delta (NULL reward, legacy estimate, phantom row);
    // 09-09 and 09-10 both belong to the 09-10 quota day at UTC+8, and the
    // higher-precedence row must win the collision.
    for (legacy_day, reward, source, recorded) in [
        (
            "2026-09-09",
            None,
            "legacy_estimate",
            "2026-09-09T17:00:00+00:00",
        ),
        (
            "2026-09-10",
            Some(2500),
            "provider_response",
            "2026-09-10T01:00:00+00:00",
        ),
        (
            "2026-09-11",
            Some(1234),
            "legacy_estimate",
            "2026-09-11T02:00:00+00:00",
        ),
        (
            "2026-09-12",
            Some(3000),
            "manual",
            "2026-09-12T05:00:00+00:00",
        ),
    ] {
        sqlx::query(
            "INSERT INTO check_in_records (account_id, check_in_date, reward_cents, source, recorded_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id.as_str())
        .bind(legacy_day)
        .bind(reward)
        .bind(source)
        .bind(recorded)
        .execute(&pool)
        .await
        .unwrap();
    }

    sqlx::raw_sql(include_str!(
        "../migrations/20260911000003_rebuild_check_in_evidence.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();

    let rows = repo
        .list_evidence_in_range(&id, day("2026-09-01"), day("2026-09-30"))
        .await
        .unwrap();
    assert_eq!(rows.len(), 3, "the colliding days collapse into one");

    // 09-09 + 09-10 -> 09-10, and the provider response wins.
    assert_eq!(rows[0].quota_day(), day("2026-09-10"));
    assert_eq!(rows[0].source(), CheckInSource::ProviderResponse);
    assert_eq!(rows[0].reward(), Some(25.0));
    // The legacy estimate keeps its amount but is labelled inferred.
    assert_eq!(rows[1].quota_day(), day("2026-09-11"));
    assert!(rows[1].reward_is_estimate());
    // Manual stays manual.
    assert_eq!(rows[2].source(), CheckInSource::Manual);
    assert_eq!(
        rows[2].confidence(),
        neuradock_domain::balance_history::CheckInConfidence::Manual
    );
}

#[test]
fn invalid_reward_values_are_rejected() {
    for value in [f64::NAN, f64::INFINITY, -1.0] {
        assert!(CheckInRecord::new(
            AccountId::new(),
            day("2026-01-02"),
            at("2026-01-02T12:00:00Z"),
            Some(value),
            CheckInSource::ProviderResponse,
        )
        .is_err());
    }
}

#[tokio::test]
async fn the_provider_offset_shifts_the_day_bucket() {
    // Two instants on either side of local midnight for a UTC+8 provider land
    // on different quota days even though they share a UTC day.
    let (_, repo, id) = fixture().await;
    repo.save_check_in(&evidence(
        &id,
        "2026-01-03",
        at("2026-01-02T16:30:00Z"),
        Some(25.0),
        CheckInSource::ProviderResponse,
    ))
    .await
    .unwrap();

    let rows = repo.list_all_daily_summaries(&id).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].quota_day(),
        day("2026-01-03"),
        "16:30 UTC is already the next day at UTC+{OFFSET_MINUTES}"
    );
}
