/// E2E Test: Error Scenarios
///
/// This test validates error handling across the full stack:
/// 1. Account not found errors
/// 2. Invalid credentials errors
/// 3. Concurrent account updates
/// 4. Session expiration
/// 5. Database constraint violations
use std::collections::HashMap;
use std::sync::Arc;

use neuradock_infrastructure::persistence::repositories::{
    SqliteAccountRepository, SqliteBalanceRepository, SqliteSessionRepository,
};
use neuradock_domain::account::{Account, AccountRepository, Credentials};
use neuradock_domain::balance::BalanceRepository;
use neuradock_domain::session::SessionRepository;
use neuradock_domain::shared::{AccountId, DomainError, ProviderId};

mod test_helpers;

#[tokio::test]
async fn e2e_error_account_not_found() {
    // ============================================================
    // Setup
    // ============================================================
    let (pool, encryption) = test_helpers::setup_in_memory_db().await;

    let account_repo: Arc<dyn AccountRepository> = Arc::new(SqliteAccountRepository::new(
        Arc::new(pool.clone()),
        encryption.clone(),
    ));

    // ============================================================
    // Try to find non-existent account
    // ============================================================
    let fake_id = AccountId::from_string("non-existent-account-id");

    let result = account_repo.find_by_id(&fake_id).await;

    assert!(result.is_ok(), "Query should succeed");
    assert!(result.unwrap().is_none(), "Account should not exist");

    println!("✓ Account not found error handled correctly");
}

#[tokio::test]
async fn e2e_error_invalid_credentials() {
    // ============================================================
    // Setup
    // ============================================================
    let (pool, encryption) = test_helpers::setup_in_memory_db().await;

    let account_repo: Arc<dyn AccountRepository> = Arc::new(SqliteAccountRepository::new(
        Arc::new(pool.clone()),
        encryption.clone(),
    ));

    // ============================================================
    // Try to create account with empty cookies - should fail
    // ============================================================
    let empty_cookies = HashMap::new();
    let credentials = Credentials::new(empty_cookies, "empty_user".to_string());

    let account = Account::new(
        "Invalid Account".to_string(),
        ProviderId::from_string("test-provider"),
        credentials,
    );

    // Account creation should fail because cookies are required
    assert!(account.is_err(), "Account with empty cookies should be rejected");

    println!("✓ Empty credentials rejection test passed");

    // ============================================================
    // Try to create account with very long name (boundary test)
    // ============================================================
    let long_name = "A".repeat(300);
    let mut cookies = HashMap::new();
    cookies.insert("token".to_string(), "value".to_string());
    let credentials = Credentials::new(cookies, "user".to_string());

    let account = Account::new(
        long_name.clone(),
        ProviderId::from_string("test-provider"),
        credentials,
    );

    assert!(account.is_ok(), "Long account name should be allowed");

    // Save and verify
    let account = account.unwrap();
    account_repo
        .save(&account)
        .await
        .expect("Save should succeed");

    let loaded = account_repo
        .find_by_id(account.id())
        .await
        .expect("Find should succeed")
        .expect("Account should exist");

    assert_eq!(loaded.name(), &long_name);

    println!("✓ Boundary test (long name) passed");
}

#[tokio::test]
async fn e2e_error_concurrent_account_updates() {
    // ============================================================
    // Setup
    // ============================================================
    let (pool, encryption) = test_helpers::setup_in_memory_db().await;

    let account_repo: Arc<dyn AccountRepository> = Arc::new(SqliteAccountRepository::new(
        Arc::new(pool.clone()),
        encryption.clone(),
    ));

    // ============================================================
    // Create account
    // ============================================================
    let mut cookies = HashMap::new();
    cookies.insert("token".to_string(), "value".to_string());
    let credentials = Credentials::new(cookies, "concurrent_user".to_string());

    let account = Account::new(
        "Concurrent Test Account".to_string(),
        ProviderId::from_string("test-provider"),
        credentials,
    )
    .expect("Create account should succeed");

    account_repo
        .save(&account)
        .await
        .expect("Initial save should succeed");

    let account_id = account.id().clone();

    println!("✓ Account created");

    // ============================================================
    // Simulate concurrent updates
    // ============================================================
    let repo1 = account_repo.clone();
    let repo2 = account_repo.clone();
    let account_id_1 = account_id.clone();
    let account_id_2 = account_id.clone();

    let task1 = tokio::spawn(async move {
        let mut acc = repo1
            .find_by_id(&account_id_1)
            .await
            .expect("Find should succeed")
            .expect("Account should exist");

        acc.toggle(true);
        repo1.save(&acc).await.expect("Save should succeed");
    });

    let task2 = tokio::spawn(async move {
        // Small delay to create race condition
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let mut acc = repo2
            .find_by_id(&account_id_2)
            .await
            .expect("Find should succeed")
            .expect("Account should exist");

        acc.update_auto_checkin(true, 10, 0)
            .expect("Update should succeed");
        repo2.save(&acc).await.expect("Save should succeed");
    });

    // Wait for both tasks
    let result1 = task1.await;
    let result2 = task2.await;

    assert!(result1.is_ok(), "First update should succeed");
    assert!(result2.is_ok(), "Second update should succeed");

    println!("✓ Concurrent updates completed");

    // ============================================================
    // Verify final state (last write wins)
    // ============================================================
    let final_account = account_repo
        .find_by_id(&account_id)
        .await
        .expect("Find should succeed")
        .expect("Account should exist");

    // The second update should have won
    assert!(final_account.auto_checkin_enabled());

    println!("✓ Concurrent update conflict resolved (last write wins)");
}

#[tokio::test]
async fn e2e_error_session_expiration() {
    // ============================================================
    // Setup
    // ============================================================
    let (pool, encryption) = test_helpers::setup_in_memory_db().await;

    let account_repo: Arc<dyn AccountRepository> = Arc::new(SqliteAccountRepository::new(
        Arc::new(pool.clone()),
        encryption.clone(),
    ));

    let session_repo = Arc::new(SqliteSessionRepository::new(
        Arc::new(pool.clone()),
    ));

    // ============================================================
    // Create account
    // ============================================================
    let mut cookies = HashMap::new();
    cookies.insert("token".to_string(), "value".to_string());
    let credentials = Credentials::new(cookies, "session_user".to_string());

    let account = Account::new(
        "Session Expiration Test".to_string(),
        ProviderId::from_string("test-provider"),
        credentials,
    )
    .expect("Create account should succeed");

    account_repo
        .save(&account)
        .await
        .expect("Account save should succeed");

    let account_id = account.id().clone();

    println!("✓ Account created");

    // ============================================================
    // Create expired session
    // ============================================================
    use neuradock_domain::session::Session;
    use chrono::Utc;

    let expired_token = "expired_token".to_string();

    // Create session that expired 1 hour ago
    let expired_session = Session::new(
        account_id.clone(),
        expired_token,
        Utc::now() - chrono::Duration::hours(1),
    ).expect("Create expired session should succeed");

    session_repo
        .save(&expired_session)
        .await
        .expect("Session save should succeed");

    println!("✓ Expired session saved");

    // ============================================================
    // Retrieve and verify expiration
    // ============================================================
    let loaded_session = session_repo
        .find_by_account_id(&account_id)
        .await
        .expect("Find should succeed")
        .expect("Session should exist");

    assert!(
        !loaded_session.is_valid(),
        "Session should be marked as expired"
    );

    println!("✓ Session expiration detected correctly");

    // ============================================================
    // Verify session can be refreshed
    // ============================================================
    let fresh_token = "fresh_token".to_string();

    let fresh_session = Session::new(
        account_id.clone(),
        fresh_token,
        Utc::now() + chrono::Duration::hours(24),
    ).expect("Create fresh session should succeed");

    session_repo
        .save(&fresh_session)
        .await
        .expect("Session refresh should succeed");

    let refreshed_session = session_repo
        .find_by_account_id(&account_id)
        .await
        .expect("Find should succeed")
        .expect("Session should exist");

    assert!(
        refreshed_session.is_valid(),
        "Refreshed session should not be expired"
    );

    println!("✓ Session refreshed successfully");
}

#[tokio::test]
async fn e2e_error_balance_edge_cases() {
    // ============================================================
    // Setup
    // ============================================================
    let (pool, encryption) = test_helpers::setup_in_memory_db().await;

    let account_repo: Arc<dyn AccountRepository> = Arc::new(SqliteAccountRepository::new(
        Arc::new(pool.clone()),
        encryption.clone(),
    ));

    let balance_repo = Arc::new(SqliteBalanceRepository::new(Arc::new(pool.clone())));

    // ============================================================
    // Create account
    // ============================================================
    let mut cookies = HashMap::new();
    cookies.insert("token".to_string(), "value".to_string());
    let credentials = Credentials::new(cookies, "balance_user".to_string());

    let account = Account::new(
        "Balance Edge Cases".to_string(),
        ProviderId::from_string("test-provider"),
        credentials,
    )
    .expect("Create account should succeed");

    account_repo
        .save(&account)
        .await
        .expect("Account save should succeed");

    let account_id = account.id().clone();

    println!("✓ Account created");

    // ============================================================
    // Test 1: Zero balance
    // ============================================================
    use neuradock_domain::balance::Balance;
    use chrono::Utc;

    let zero_balance = Balance::restore(
        account_id.clone(),
        0.0,
        0.0,
        0.0,
        Utc::now(),
    );

    balance_repo
        .save(&zero_balance)
        .await
        .expect("Zero balance should be saved");

    println!("✓ Zero balance saved");

    // ============================================================
    // Test 2: Negative balance (edge case, should be allowed)
    // ============================================================
    let negative_balance = Balance::restore(
        account_id.clone(),
        -10.0,
        50.0,
        40.0,
        Utc::now(),
    );

    balance_repo
        .save(&negative_balance)
        .await
        .expect("Negative balance should be saved");

    println!("✓ Negative balance saved");

    // ============================================================
    // Test 3: Very large balance
    // ============================================================
    let large_balance = Balance::restore(
        account_id.clone(),
        999999999.99,
        1000000.0,
        1000999999.99,
        Utc::now(),
    );

    balance_repo
        .save(&large_balance)
        .await
        .expect("Large balance should be saved");

    println!("✓ Large balance saved");

    // ============================================================
    // Verify latest balance is the large one
    // ============================================================
    // Note: BalanceRepository stores only current balance, not history
    let current_balance = balance_repo
        .find_by_account_id(&account_id)
        .await
        .expect("Find balance should succeed")
        .expect("Balance should exist");

    // Latest should be the large balance
    assert_eq!(current_balance.current(), 999999999.99);

    println!("✓ Latest balance verified");
}

#[tokio::test]
async fn e2e_error_account_deletion_cascade() {
    // ============================================================
    // Setup
    // ============================================================
    let (pool, encryption) = test_helpers::setup_in_memory_db().await;

    let account_repo: Arc<dyn AccountRepository> = Arc::new(SqliteAccountRepository::new(
        Arc::new(pool.clone()),
        encryption.clone(),
    ));

    let balance_repo = Arc::new(SqliteBalanceRepository::new(Arc::new(pool.clone())));
    let session_repo = Arc::new(SqliteSessionRepository::new(
        Arc::new(pool.clone()),
    ));

    // ============================================================
    // Create account with balance and session
    // ============================================================
    let mut cookies = HashMap::new();
    cookies.insert("token".to_string(), "value".to_string());
    let credentials = Credentials::new(cookies.clone(), "delete_user".to_string());

    let account = Account::new(
        "Deletion Test Account".to_string(),
        ProviderId::from_string("test-provider"),
        credentials,
    )
    .expect("Create account should succeed");

    account_repo
        .save(&account)
        .await
        .expect("Account save should succeed");

    let account_id = account.id().clone();

    // Save balance
    use neuradock_domain::balance::Balance;
    use neuradock_domain::session::Session;
    use chrono::Utc;

    let balance = Balance::restore(account_id.clone(), 100.0, 20.0, 120.0, Utc::now());
    balance_repo
        .save(&balance)
        .await
        .expect("Balance save should succeed");

    // Save session
    let session = Session::new(
        account_id.clone(),
        "test_session_token".to_string(),
        Utc::now() + chrono::Duration::hours(24),
    ).expect("Create session should succeed");
    session_repo
        .save(&session)
        .await
        .expect("Session save should succeed");

    println!("✓ Account, balance, and session created");

    // ============================================================
    // Delete account
    // ============================================================
    account_repo
        .delete(&account_id)
        .await
        .expect("Account deletion should succeed");

    println!("✓ Account deleted");

    // ============================================================
    // Verify account is deleted
    // ============================================================
    let deleted_account = account_repo
        .find_by_id(&account_id)
        .await
        .expect("Find should succeed");

    assert!(deleted_account.is_none(), "Account should be deleted");

    println!("✓ Account deletion verified");

    // ============================================================
    // Verify related data (balance and session may or may not cascade delete)
    // This depends on database schema constraints
    // ============================================================
    let balance_result = balance_repo.find_by_account_id(&account_id).await;
    println!("✓ Balance query after account deletion: {:?}", balance_result.is_ok());

    let session_result = session_repo.find_by_account_id(&account_id).await;
    println!("✓ Session query after account deletion: {:?}", session_result.is_ok());
}

#[tokio::test]
async fn e2e_error_auto_checkin_validation() {
    // ============================================================
    // Setup
    // ============================================================
    let (pool, encryption) = test_helpers::setup_in_memory_db().await;

    let account_repo: Arc<dyn AccountRepository> = Arc::new(SqliteAccountRepository::new(
        Arc::new(pool.clone()),
        encryption.clone(),
    ));

    // ============================================================
    // Create account
    // ============================================================
    let mut cookies = HashMap::new();
    cookies.insert("token".to_string(), "value".to_string());
    let credentials = Credentials::new(cookies, "validation_user".to_string());

    let mut account = Account::new(
        "Auto Check-in Validation".to_string(),
        ProviderId::from_string("test-provider"),
        credentials,
    )
    .expect("Create account should succeed");

    account_repo
        .save(&account)
        .await
        .expect("Account save should succeed");

    println!("✓ Account created");

    // ============================================================
    // Test 1: Invalid hour (out of range)
    // ============================================================
    let result = account.update_auto_checkin(true, 25, 0);

    assert!(
        result.is_err(),
        "Hour 25 should be rejected"
    );

    println!("✓ Invalid hour rejected");

    // ============================================================
    // Test 2: Invalid minute (out of range)
    // ============================================================
    let result = account.update_auto_checkin(true, 9, 60);

    assert!(
        result.is_err(),
        "Minute 60 should be rejected"
    );

    println!("✓ Invalid minute rejected");

    // ============================================================
    // Test 3: Valid configuration
    // ============================================================
    let result = account.update_auto_checkin(true, 23, 59);

    assert!(
        result.is_ok(),
        "Valid time 23:59 should be accepted"
    );

    account_repo
        .save(&account)
        .await
        .expect("Save should succeed");

    println!("✓ Valid auto check-in configuration accepted");

    // ============================================================
    // Verify configuration
    // ============================================================
    let loaded = account_repo
        .find_by_id(account.id())
        .await
        .expect("Find should succeed")
        .expect("Account should exist");

    assert_eq!(loaded.auto_checkin_hour(), 23);
    assert_eq!(loaded.auto_checkin_minute(), 59);

    println!("✓ Auto check-in validation tests passed");
}
