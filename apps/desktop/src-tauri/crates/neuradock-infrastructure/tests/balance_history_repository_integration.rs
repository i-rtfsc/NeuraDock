use chrono::{Duration, Utc};
use std::sync::Arc;

use neuradock_domain::balance_history::{BalanceHistoryRecord, BalanceHistoryRepository};
use neuradock_domain::shared::AccountId;
use neuradock_infrastructure::persistence::repositories::SqliteBalanceHistoryRepository;

mod test_helpers;

#[tokio::test]
async fn balance_history_repo_save_and_find_latest_integration() {
    let (pool, _encryption) = test_helpers::setup_in_memory_db().await;

    let repo = SqliteBalanceHistoryRepository::new(Arc::new(pool.clone()));

    let account_id = AccountId::new();
    sqlx::query("INSERT OR IGNORE INTO accounts (id, name, provider_id, cookies, api_user, enabled, created_at) VALUES (?1, ?2, ?3, ?4, ?5, 1, datetime('now'))")
        .bind(account_id.as_str())
        .bind("Test Account")
        .bind("test-provider")
        .bind("{}")
        .bind("api_user")
        .execute(&pool)
        .await
        .expect("insert account");

    let yesterday = Utc::now() - Duration::days(1);
    let today = Utc::now();

    let older = BalanceHistoryRecord::new(
        "older".to_string(),
        account_id.clone(),
        10.0,
        1.0,
        0.5,
        yesterday,
    )
    .expect("create older record");
    repo.save(&older).await.expect("save older");

    let newer = BalanceHistoryRecord::new(
        "newer".to_string(),
        account_id.clone(),
        20.0,
        2.0,
        1.5,
        today,
    )
    .expect("create newer record");
    repo.save(&newer).await.expect("save newer");

    let latest = repo
        .find_latest_by_account_id(&account_id)
        .await
        .expect("find latest")
        .expect("latest should exist");

    assert_eq!(latest.id(), "newer");
    assert_eq!(latest.current_balance(), 20.0);

    let updated = BalanceHistoryRecord::new(
        "newer".to_string(),
        account_id.clone(),
        25.0,
        2.0,
        1.5,
        Utc::now() + Duration::seconds(1),
    )
    .expect("create updated record");
    repo.save(&updated).await.expect("save updated");

    let latest = repo
        .find_latest_by_account_id(&account_id)
        .await
        .expect("find latest after update")
        .expect("latest should exist");

    assert_eq!(latest.id(), "newer");
    assert_eq!(latest.current_balance(), 25.0);
}

