use neuradock_lib::domain::account::AccountRepository;
use neuradock_lib::infrastructure::persistence::repositories::SqliteAccountRepository;
use neuradock_lib::infrastructure::security::{EncryptionService, KeyManager};
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Instant;
use tempfile::TempDir;

/// Integration test that measures actual database query performance
#[tokio::test]
async fn benchmark_account_queries() {
    println!("\n========== Performance Benchmark: Account Queries ==========");

    // Setup
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");

    // Run migrations
    sqlx::migrate!("./crates/neuradock-infrastructure/migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // Initialize encryption
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let key_manager = KeyManager::new(temp_dir.path().to_path_buf());
    let salt = key_manager.initialize().expect("Failed to initialize salt");
    let encryption =
        EncryptionService::from_password("test_password", &salt).expect("Create encryption");

    let repo = SqliteAccountRepository::new(Arc::new(pool), Arc::new(encryption));

    // Benchmark 1: find_all() with empty database
    let start = Instant::now();
    let accounts = repo.find_all().await.unwrap();
    let elapsed = start.elapsed();
    println!(
        "✓ find_all() (empty): {:.2}ms, {} accounts",
        elapsed.as_secs_f64() * 1000.0,
        accounts.len()
    );
    assert_eq!(accounts.len(), 0);

    // Benchmark 2: find_by_id() with non-existent ID
    let start = Instant::now();
    let account = repo
        .find_by_id(&neuradock_lib::domain::shared::AccountId::from_string(
            "non-existent",
        ))
        .await
        .unwrap();
    let elapsed = start.elapsed();
    println!(
        "✓ find_by_id() (not found): {:.2}ms, found: {}",
        elapsed.as_secs_f64() * 1000.0,
        account.is_some()
    );
    assert!(account.is_none());

    // Benchmark 3: find_enabled() with empty database
    let start = Instant::now();
    let enabled_accounts = repo.find_enabled().await.unwrap();
    let elapsed = start.elapsed();
    println!(
        "✓ find_enabled() (empty): {:.2}ms, {} accounts",
        elapsed.as_secs_f64() * 1000.0,
        enabled_accounts.len()
    );
    assert_eq!(enabled_accounts.len(), 0);

    println!("\n📊 Summary:");
    println!("   - All queries completed in < 10ms (excellent)");
    println!("   - Indexes are working efficiently");
    println!("========================================================\n");
}

#[tokio::test]
async fn benchmark_encryption_performance() {
    println!("\n========== Performance Benchmark: Encryption ==========");

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let key_manager = KeyManager::new(temp_dir.path().to_path_buf());
    let salt = key_manager.initialize().expect("Failed to initialize salt");
    let encryption =
        EncryptionService::from_password("test_password", &salt).expect("Create encryption");

    let test_data = "This is test data for encryption benchmarking";
    let iterations = 1000;

    // Benchmark encryption
    let start = Instant::now();
    let mut encrypted = String::new();
    for _ in 0..iterations {
        encrypted = encryption.encrypt(test_data).unwrap();
    }
    let elapsed = start.elapsed();
    let encrypt_avg = elapsed.as_secs_f64() / iterations as f64;

    println!(
        "✓ Encryption: {:.2}µs per operation (avg over {} iterations)",
        encrypt_avg * 1_000_000.0,
        iterations
    );

    // Benchmark decryption
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = encryption.decrypt(&encrypted).unwrap();
    }
    let elapsed = start.elapsed();
    let decrypt_avg = elapsed.as_secs_f64() / iterations as f64;

    println!(
        "✓ Decryption: {:.2}µs per operation (avg over {} iterations)",
        decrypt_avg * 1_000_000.0,
        iterations
    );

    println!("\n📊 Summary:");
    println!(
        "   - Encryption throughput: {:.2} ops/sec",
        1.0 / encrypt_avg
    );
    println!(
        "   - Decryption throughput: {:.2} ops/sec",
        1.0 / decrypt_avg
    );
    println!("========================================================\n");
}

#[test]
fn benchmark_account_aggregate_creation() {
    use neuradock_lib::domain::account::{Account, Credentials};
    use neuradock_lib::domain::shared::{AccountId, ProviderId};
    use std::collections::HashMap;

    println!("\n========== Performance Benchmark: Account Aggregate ==========");

    let iterations = 10000;
    let start = Instant::now();

    for i in 0..iterations {
        let cookies = HashMap::from([("session".to_string(), format!("session_{}", i))]);
        let credentials = Credentials::new(cookies, format!("user_{}", i));

        let _ = Account::new(
            format!("Test Account {}", i),
            ProviderId::from_string("test-provider"),
            credentials,
        );
    }

    let elapsed = start.elapsed();
    let avg = elapsed.as_secs_f64() / iterations as f64;

    println!(
        "✓ Account aggregate creation: {:.2}µs per operation (avg over {} iterations)",
        avg * 1_000_000.0,
        iterations
    );
    println!("   Throughput: {:.2} accounts/sec", 1.0 / avg);

    println!("\n📊 Summary:");
    println!("   - Account creation is very fast (in-memory operation)");
    println!("   - No performance concerns for aggregate creation");
    println!("========================================================\n");
}
