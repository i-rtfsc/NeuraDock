use chrono::{Duration, Utc};
use sqlx::SqlitePool;
use std::sync::Arc;

use neuradock_domain::session::{Session, SessionRepository};
use neuradock_domain::shared::AccountId;
use neuradock_infrastructure::persistence::repositories::SqliteSessionRepository;

mod test_helpers;

#[tokio::test]
async fn session_repo_save_find_and_valid_integration() {
    let (pool, _encryption) = test_helpers::setup_in_memory_db().await;

    let repo = SqliteSessionRepository::new(Arc::new(pool.clone()));

    let account_id = AccountId::new();
    // ensure account exists for FK
    sqlx::query("INSERT OR IGNORE INTO accounts (id, name, provider_id, cookies, api_user, enabled, created_at) VALUES (?1, ?2, ?3, ?4, ?5, 1, datetime('now'))")
        .bind(account_id.as_str())
        .bind("Test Account")
        .bind("test-provider")
        .bind("{}")
        .bind("api_user")
        .execute(&pool)
        .await
        .expect("insert account");
    let token = "token_abc".to_string();
    let expires_at = Utc::now() + Duration::hours(24);

    let mut session =
        Session::new(account_id.clone(), token.clone(), expires_at).expect("create session");

    repo.save(&session).await.expect("save session");

    // find by account id
    let fetched = repo
        .find_by_account_id(&account_id)
        .await
        .expect("find")
        .expect("should exist");

    assert_eq!(fetched.account_id().as_str(), account_id.as_str());
    assert_eq!(fetched.token(), token);

    // valid sessions should include our session
    let valids = repo.find_valid_sessions().await.expect("find valid");
    let ids: Vec<String> = valids
        .into_iter()
        .map(|s| s.account_id().as_str().to_string())
        .collect();
    assert!(ids.contains(&account_id.as_str().to_string()));

    // expire and save
    session.expire();
    repo.save(&session).await.expect("save expired");

    let valids_after = repo
        .find_valid_sessions()
        .await
        .expect("find valid after expire");
    let ids_after: Vec<String> = valids_after
        .into_iter()
        .map(|s| s.account_id().as_str().to_string())
        .collect();
    assert!(!ids_after.contains(&account_id.as_str().to_string()));

    // delete
    repo.delete(&account_id).await.expect("delete");
    let not_found = repo
        .find_by_account_id(&account_id)
        .await
        .expect("find after delete");
    assert!(not_found.is_none());
}
