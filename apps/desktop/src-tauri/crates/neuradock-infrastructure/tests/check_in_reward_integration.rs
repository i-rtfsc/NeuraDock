use chrono::{DateTime, Duration, Utc};
use neuradock_domain::balance_history::{
    BalanceHistoryRecord, BalanceHistoryRepository, CheckInRecord,
};
use neuradock_domain::shared::AccountId;
use neuradock_infrastructure::persistence::repositories::SqliteBalanceHistoryRepository;
use sqlx::SqlitePool;
use std::sync::Arc;

mod test_helpers;

async fn fixture() -> (SqlitePool, SqliteBalanceHistoryRepository, AccountId) {
    let (pool, _) = test_helpers::setup_in_memory_db().await;
    let id = AccountId::new();
    sqlx::query("INSERT INTO accounts (id,name,provider_id,cookies,api_user,created_at) VALUES (?,'Test account','test-provider','test-cookie','test-user',datetime('now'))")
        .bind(id.as_str()).execute(&pool).await.unwrap();
    let repo = SqliteBalanceHistoryRepository::new(Arc::new(pool.clone()));
    (pool, repo, id)
}

fn timestamp() -> DateTime<Utc> {
    "2026-01-02T12:00:00Z".parse().unwrap()
}

fn snapshot(
    id: &AccountId,
    current: f64,
    consumed: f64,
    at: DateTime<Utc>,
) -> BalanceHistoryRecord {
    BalanceHistoryRecord::new(
        format!("{}-{}", id.as_str(), at.date_naive()),
        id.clone(),
        current,
        consumed,
        current + consumed,
        at,
    )
    .unwrap()
}

#[tokio::test]
async fn concurrent_usage_refresh_and_retries_cannot_change_a_confirmed_reward() {
    let (pool, repo, id) = fixture().await;
    let now = timestamp();
    repo.save(&snapshot(&id, 100.0, 200.0, now - Duration::days(1)))
        .await
        .unwrap();
    let reward = CheckInRecord::new(id.clone(), now, Some(25.0), "provider_rule").unwrap();
    assert!(repo.save_check_in(&reward).await.unwrap());

    // The agent's 1.56 pre-charge is visible before used_quota settles.
    let pending_usage = snapshot(&id, 123.44, 200.0, now);
    let (refresh, duplicate) = tokio::join!(repo.save(&pending_usage), repo.save_check_in(&reward));
    refresh.unwrap();
    assert!(!duplicate.unwrap());
    let daily = repo
        .find_daily_summary(&id, now.date_naive())
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
            .find_daily_summary(&id, now.date_naive())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(daily.check_in_reward(), Some(25.0));
        assert_eq!(daily.daily_balance(), current);
    }
    let unknown = CheckInRecord::new(id.clone(), now, None, "confirmed_unknown").unwrap();
    assert!(!repo.save_check_in(&unknown).await.unwrap());
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
    let now = timestamp();
    repo.save(&snapshot(&id, 1000.0, 200.0, now)).await.unwrap();
    let daily = repo
        .find_daily_summary(&id, now.date_naive())
        .await
        .unwrap()
        .unwrap();
    assert!(!daily.is_checked_in());
    assert_eq!(daily.check_in_reward(), None);
}

#[tokio::test]
async fn rewards_survive_missing_balance_responses_and_unknown_amounts_stay_unknown() {
    let (_, repo, id) = fixture().await;
    let now = timestamp();
    repo.save_check_in(
        &CheckInRecord::new(id.clone(), now, Some(25.0), "provider_response").unwrap(),
    )
    .await
    .unwrap();
    repo.save_check_in(
        &CheckInRecord::new(
            id.clone(),
            now + Duration::days(1),
            None,
            "confirmed_unknown",
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let rows = repo.list_all_daily_summaries(&id).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].check_in_reward(), Some(25.0));
    assert!(rows[1].is_checked_in());
    assert_eq!(rows[1].check_in_reward(), None);
    assert!(!rows[1].reward_is_estimate());
}

#[tokio::test]
async fn migration_freezes_legacy_estimates_without_inventing_a_first_day_reward() {
    let (pool, repo, id) = fixture().await;
    sqlx::query("DROP TABLE check_in_records")
        .execute(&pool)
        .await
        .unwrap();
    let now = timestamp();
    repo.save(&snapshot(&id, 1000.0, 200.0, now - Duration::days(1)))
        .await
        .unwrap();
    repo.save(&snapshot(&id, 1023.44, 200.0, now))
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../migrations/20260910000001_create_check_in_records.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let rows = repo.list_all_daily_summaries(&id).await.unwrap();
    assert_eq!(rows[0].check_in_reward(), None);
    assert_eq!(rows[1].check_in_reward(), Some(23.44));
    assert!(rows[1].reward_is_estimate());

    repo.save(&snapshot(&id, 3000.0, 300.0, now + Duration::seconds(1)))
        .await
        .unwrap();
    assert_eq!(
        repo.find_daily_summary(&id, now.date_naive())
            .await
            .unwrap()
            .unwrap()
            .check_in_reward(),
        Some(23.44)
    );
    assert!(repo
        .save_check_in(&CheckInRecord::new(id.clone(), now, Some(25.0), "manual").unwrap())
        .await
        .unwrap());
    assert!(!repo
        .save_check_in(&CheckInRecord::new(id.clone(), now, Some(2.0), "provider_rule").unwrap())
        .await
        .unwrap());
    let daily = repo
        .find_daily_summary(&id, now.date_naive())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(daily.check_in_reward(), Some(25.0));
    assert!(!daily.reward_is_estimate());
}

#[test]
fn invalid_reward_values_are_rejected() {
    for value in [f64::NAN, f64::INFINITY, -1.0] {
        assert!(CheckInRecord::new(
            AccountId::new(),
            timestamp(),
            Some(value),
            "provider_response"
        )
        .is_err());
    }
}
