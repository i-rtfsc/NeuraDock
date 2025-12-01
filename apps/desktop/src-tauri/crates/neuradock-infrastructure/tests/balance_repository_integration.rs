use sqlx::SqlitePool;
use std::sync::Arc;
use chrono::{Utc, Duration};

use neuradock_infrastructure::persistence::repositories::SqliteBalanceRepository;
use neuradock_domain::balance::{Balance, BalanceRepository};
use neuradock_domain::shared::AccountId;

mod test_helpers;

#[tokio::test]
async fn balance_repo_save_find_and_stale_integration() {
    let (pool, _encryption) = test_helpers::setup_in_memory_db().await;

    let repo = SqliteBalanceRepository::new(Arc::new(pool.clone()));

    // create a fresh balance and save
    let account_id = AccountId::new();
    // ensure account exists to satisfy foreign key
    sqlx::query("INSERT OR IGNORE INTO accounts (id, name, provider_id, cookies, api_user, enabled, created_at) VALUES (?1, ?2, ?3, ?4, ?5, 1, datetime('now'))")
        .bind(account_id.as_str())
        .bind("Test Account")
        .bind("test-provider")
        .bind("{}")
        .bind("api_user")
        .execute(&pool)
        .await
        .expect("insert account");
    let balance = Balance::new(account_id.clone(), 100.0).expect("create balance");

    repo.save(&balance).await.expect("save balance");

    // find by account id
    let fetched = repo
        .find_by_account_id(&account_id)
        .await
        .expect("find")
        .expect("should exist");

    assert_eq!(fetched.account_id().as_str(), account_id.as_str());
    assert_eq!(fetched.current(), 100.0);

    // create a stale balance (older last_checked_at) using restore
    let stale_account = AccountId::new();
    // ensure stale account exists
    sqlx::query("INSERT OR IGNORE INTO accounts (id, name, provider_id, cookies, api_user, enabled, created_at) VALUES (?1, ?2, ?3, ?4, ?5, 1, datetime('now'))")
        .bind(stale_account.as_str())
        .bind("Stale Account")
        .bind("test-provider")
        .bind("{}")
        .bind("api_user")
        .execute(&pool)
        .await
        .expect("insert stale account");
    let old_time = Utc::now() - Duration::hours(48);
    let stale_balance = Balance::restore(stale_account.clone(), 50.0, 0.0, 0.0, old_time);

    repo.save(&stale_balance).await.expect("save stale");

    // find stale balances older than 24 hours
    let stale = repo.find_stale_balances(24).await.expect("find stale");
    let ids: Vec<String> = stale.into_iter().map(|b| b.account_id().as_str().to_string()).collect();

    assert!(ids.contains(&stale_account.as_str().to_string()));

    // delete the first balance
    repo.delete(&account_id).await.expect("delete");
    let not_found = repo.find_by_account_id(&account_id).await.expect("find after delete");
    assert!(not_found.is_none());
}
