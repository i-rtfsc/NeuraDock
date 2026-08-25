use neuradock_infrastructure::security::{encrypt_codex_credentials, EncryptionService};
use sqlx::Row;

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
