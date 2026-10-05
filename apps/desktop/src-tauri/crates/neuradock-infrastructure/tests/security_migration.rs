use neuradock_infrastructure::security::{
    encrypt_codex_credentials, encryption_key_migration_complete, initialize_database_encryption,
    reencrypt_legacy_records, EncryptionService, KeyManager,
};
use sqlx::{Row, SqlitePool};
use tempfile::TempDir;

mod test_helpers;

#[tokio::test]
async fn codex_credentials_migrate_from_plaintext_to_encrypted_storage() {
    let (pool, legacy) = test_helpers::setup_in_memory_db().await;
    sqlx::query(
        "INSERT INTO codex_accounts (id, email, password, access_token, created_at, updated_at)
         VALUES ('account-1', 'user@example.com', 'plain-password', 'plain-access', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let key = [7u8; 32];
    let current = EncryptionService::from_key(&key).unwrap();
    let migrated = encrypt_codex_credentials(&pool, &current).await.unwrap();
    assert_eq!(migrated, 2);

    let row =
        sqlx::query("SELECT password, access_token FROM codex_accounts WHERE id = 'account-1'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let password: String = row.get("password");
    let access_token: String = row.get("access_token");

    assert!(legacy.decrypt(&password).is_err());
    assert!(legacy.decrypt(&access_token).is_err());
    assert_eq!(current.decrypt(&password).unwrap(), "plain-password");
    assert_eq!(current.decrypt(&access_token).unwrap(), "plain-access");
    assert_eq!(encrypt_codex_credentials(&pool, &current).await.unwrap(), 0);
}

async fn insert_account(pool: &SqlitePool, id: &str, encryption: &EncryptionService) -> String {
    let cookies = encryption.encrypt(r#"{"session":"test-session"}"#).unwrap();
    sqlx::query(
        "INSERT INTO accounts (id, name, provider_id, cookies, api_user, created_at)
         VALUES (?, 'Test account', 'test-provider', ?, ?, '2026-01-01T00:00:00Z')",
    )
    .bind(id)
    .bind(&cookies)
    .bind(encryption.encrypt("test-user").unwrap())
    .execute(pool)
    .await
    .unwrap();
    cookies
}

async fn account_cookies(pool: &SqlitePool, id: &str) -> String {
    sqlx::query_scalar("SELECT cookies FROM accounts WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn legacy_encryption(manager: &KeyManager) -> EncryptionService {
    EncryptionService::from_password(
        "neuradock_default_password_2024",
        &manager.initialize().unwrap(),
    )
    .unwrap()
}

#[tokio::test]
async fn legacy_import_migrates_even_when_a_master_key_already_exists() {
    let (pool, _) = test_helpers::setup_in_memory_db().await;
    let dir = TempDir::new().unwrap();
    let manager = KeyManager::new(dir.path().to_owned());
    let key = manager.initialize_master_key().unwrap().unwrap();
    let current = EncryptionService::from_key(&key).unwrap();
    // Importing old data adds a salt beside the pre-existing random key.
    let legacy = legacy_encryption(&manager);
    insert_account(&pool, "legacy", &legacy).await;
    let current_cookies = insert_account(&pool, "current", &current).await;
    let current_password = current.encrypt("codex-password").unwrap();
    sqlx::query(
        "INSERT INTO codex_accounts (id, email, password, created_at, updated_at)
         VALUES ('codex', 'user@example.com', ?, '2026-01-01', '2026-01-01')",
    )
    .bind(&current_password)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO app_security_metadata (key, value) VALUES ('codex_credentials_version', '1')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let encryption = initialize_database_encryption(&pool, &manager)
        .await
        .unwrap();
    assert_eq!(
        encryption
            .decrypt(&account_cookies(&pool, "legacy").await)
            .unwrap(),
        r#"{"session":"test-session"}"#
    );
    assert_eq!(account_cookies(&pool, "current").await, current_cookies);
    let password: String =
        sqlx::query_scalar("SELECT password FROM codex_accounts WHERE id = 'codex'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(password, current_password);
    assert_eq!(encryption.decrypt(&password).unwrap(), "codex-password");
    assert_eq!(manager.initialize_master_key().unwrap(), Some(key));
    assert!(encryption_key_migration_complete(&pool).await.unwrap());

    let migrated_cookies = account_cookies(&pool, "legacy").await;
    initialize_database_encryption(&pool, &manager)
        .await
        .unwrap();
    assert_eq!(account_cookies(&pool, "legacy").await, migrated_cookies);
}

#[tokio::test]
async fn development_and_production_databases_share_a_stable_migration_key() {
    let (development, _) = test_helpers::setup_in_memory_db().await;
    let (production, _) = test_helpers::setup_in_memory_db().await;
    let dir = TempDir::new().unwrap();
    let manager = KeyManager::new(dir.path().to_owned());
    let legacy = legacy_encryption(&manager);
    insert_account(&development, "dev", &legacy).await;
    insert_account(&production, "prod", &legacy).await;

    initialize_database_encryption(&development, &manager)
        .await
        .unwrap();
    let key = manager.initialize_master_key().unwrap().unwrap();
    let encryption = initialize_database_encryption(&production, &manager)
        .await
        .unwrap();
    assert_eq!(manager.initialize_master_key().unwrap(), Some(key));
    for (pool, id) in [(&development, "dev"), (&production, "prod")] {
        assert_eq!(
            encryption
                .decrypt(&account_cookies(pool, id).await)
                .unwrap(),
            r#"{"session":"test-session"}"#
        );
        assert!(encryption_key_migration_complete(pool).await.unwrap());
    }
}

#[tokio::test]
async fn committed_migration_recovers_its_pending_key_after_restart() {
    let (pool, _) = test_helpers::setup_in_memory_db().await;
    let dir = TempDir::new().unwrap();
    let manager = KeyManager::new(dir.path().to_owned());
    let legacy = legacy_encryption(&manager);
    insert_account(&pool, "legacy", &legacy).await;
    let key = [7u8; 32];
    manager.save_pending_master_key(&key).unwrap();
    let current = EncryptionService::from_key(&key).unwrap();
    reencrypt_legacy_records(&pool, &legacy, &current)
        .await
        .unwrap();

    let encryption = initialize_database_encryption(&pool, &manager)
        .await
        .unwrap();
    assert_eq!(manager.initialize_master_key().unwrap(), Some(key));
    assert_eq!(manager.load_pending_master_key().unwrap(), None);
    assert_eq!(
        encryption
            .decrypt(&account_cookies(&pool, "legacy").await)
            .unwrap(),
        r#"{"session":"test-session"}"#
    );
}

#[tokio::test]
async fn corrupt_encrypted_credentials_roll_back_the_entire_migration() {
    let (pool, _) = test_helpers::setup_in_memory_db().await;
    let dir = TempDir::new().unwrap();
    let manager = KeyManager::new(dir.path().to_owned());
    let key = manager.initialize_master_key().unwrap().unwrap();
    let legacy = legacy_encryption(&manager);
    let original = insert_account(&pool, "legacy", &legacy).await;
    sqlx::query(
        "INSERT INTO codex_accounts (id, email, password, created_at, updated_at)
         VALUES ('corrupt', 'user@example.com', 'invalid-ciphertext', '2026-01-01', '2026-01-01')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO app_security_metadata (key, value) VALUES ('codex_credentials_version', '1')",
    )
    .execute(&pool)
    .await
    .unwrap();

    assert!(initialize_database_encryption(&pool, &manager)
        .await
        .is_err());
    assert_eq!(account_cookies(&pool, "legacy").await, original);
    assert!(!encryption_key_migration_complete(&pool).await.unwrap());
    assert_eq!(manager.initialize_master_key().unwrap(), Some(key));
    let password: String =
        sqlx::query_scalar("SELECT password FROM codex_accounts WHERE id = 'corrupt'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(password, "invalid-ciphertext");
}

#[tokio::test]
async fn new_database_records_its_key_version_and_keeps_credentials_readable() {
    let (pool, _) = test_helpers::setup_in_memory_db().await;
    let dir = TempDir::new().unwrap();
    let manager = KeyManager::new(dir.path().to_owned());
    let encryption = initialize_database_encryption(&pool, &manager)
        .await
        .unwrap();
    let cookies = insert_account(&pool, "new", &encryption).await;
    let restarted = initialize_database_encryption(&pool, &manager)
        .await
        .unwrap();

    assert!(!manager.salt_path().exists());
    assert!(encryption_key_migration_complete(&pool).await.unwrap());
    assert_eq!(
        restarted.decrypt(&cookies).unwrap(),
        r#"{"session":"test-session"}"#
    );
}
