use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;

use neuradock_domain::account::{Account, AccountRepository, Credentials};
use neuradock_domain::shared::ProviderId;
use neuradock_infrastructure::persistence::repositories::SqliteAccountRepository;
use neuradock_infrastructure::security::EncryptionService;

mod test_helpers;

#[tokio::test]
async fn account_repo_save_and_find_integration() {
    let (pool, encryption) = test_helpers::setup_in_memory_db().await;

    let repo = SqliteAccountRepository::new(Arc::new(pool.clone()), encryption);

    // Build domain account
    let mut cookies = HashMap::new();
    cookies.insert("session".to_string(), "abc123".to_string());
    let credentials = Credentials::new(cookies, "api_user_1".to_string());

    let account = Account::new(
        "Integration Account".to_string(),
        ProviderId::from_string("test-provider"),
        credentials,
    )
    .expect("Create account aggregate");

    // Save
    repo.save(&account).await.expect("Save account");

    // Find
    let found = repo
        .find_by_id(account.id())
        .await
        .expect("Find account")
        .expect("Account should be found");

    assert_eq!(found.name(), account.name());
    assert_eq!(found.provider_id().as_str(), account.provider_id().as_str());
}
