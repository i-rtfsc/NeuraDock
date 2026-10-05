use anyhow::Context;
use sqlx::{Row, SqlitePool};

use super::{EncryptionService, KeyManager};

/// Select the key for this database and migrate any legacy records before
/// repositories are opened. Several databases can share one installation key,
/// and an imported legacy database can arrive after that key already exists.
pub async fn initialize_database_encryption(
    pool: &SqlitePool,
    key_manager: &KeyManager,
) -> anyhow::Result<EncryptionService> {
    let master_key = key_manager.initialize_master_key()?;
    let pending_key = key_manager.load_pending_master_key()?;
    let migration_complete = encryption_key_migration_complete(pool).await?;

    if master_key.is_none() && migration_complete {
        let key = pending_key.context("Migrated database has no encryption master key")?;
        key_manager.promote_pending_master_key()?;
        return Ok(EncryptionService::from_key(&key)?);
    }

    // Reuse an existing (or staged) key. Rotating it for a second database
    // would make the first database unreadable.
    let key = match master_key.or(pending_key) {
        Some(key) => key,
        None => key_manager.generate_migration_key()?,
    };
    let current = EncryptionService::from_key(&key)?;

    if !migration_complete {
        let legacy = if key_manager.salt_path().exists() {
            EncryptionService::from_password(
                "neuradock_default_password_2024",
                &key_manager.initialize()?,
            )?
        } else {
            EncryptionService::from_key(&key)?
        };

        if master_key.is_none() {
            key_manager.save_pending_master_key(&key)?;
        }
        let migrated = reencrypt_legacy_records(pool, &legacy, &current)
            .await
            .context("Failed to migrate encrypted records")?;
        if master_key.is_none() {
            key_manager.promote_pending_master_key()?;
        }
        tracing::info!(
            "Verified and migrated {} encrypted database fields",
            migrated
        );
    }

    Ok(current)
}

fn reencrypt_value(
    value: &str,
    legacy: &EncryptionService,
    current: &EncryptionService,
) -> anyhow::Result<String> {
    if current.decrypt(value).is_ok() {
        return Ok(value.to_owned());
    }
    Ok(current.encrypt(&legacy.decrypt(value)?)?)
}

pub async fn reencrypt_legacy_records(
    pool: &SqlitePool,
    legacy: &EncryptionService,
    current: &EncryptionService,
) -> anyhow::Result<u64> {
    let mut tx = pool.begin().await?;
    let mut migrated = 0;

    let accounts = sqlx::query("SELECT id, cookies, api_user FROM accounts")
        .fetch_all(&mut *tx)
        .await?;
    for row in accounts {
        let id: String = row.try_get("id")?;
        let cookies: String = row.try_get("cookies")?;
        let api_user: String = row.try_get("api_user")?;
        let cookies = reencrypt_value(&cookies, legacy, current)?;
        let api_user = reencrypt_value(&api_user, legacy, current)?;
        sqlx::query("UPDATE accounts SET cookies = ?, api_user = ? WHERE id = ?")
            .bind(cookies)
            .bind(api_user)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        migrated += 2;
    }

    let keys = sqlx::query("SELECT id, api_key FROM independent_api_keys")
        .fetch_all(&mut *tx)
        .await?;
    for row in keys {
        let id: i64 = row.try_get("id")?;
        let api_key: String = row.try_get("api_key")?;
        let api_key = reencrypt_value(&api_key, legacy, current)?;
        sqlx::query("UPDATE independent_api_keys SET api_key = ? WHERE id = ?")
            .bind(api_key)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        migrated += 1;
    }

    let connections = sqlx::query(
        "SELECT id, encrypted_secret FROM developer_tool_connections WHERE encrypted_secret IS NOT NULL",
    )
    .fetch_all(&mut *tx)
    .await?;
    for row in connections {
        let id: String = row.try_get("id")?;
        let secret: String = row.try_get("encrypted_secret")?;
        let secret = reencrypt_value(&secret, legacy, current)?;
        sqlx::query("UPDATE developer_tool_connections SET encrypted_secret = ? WHERE id = ?")
            .bind(secret)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        migrated += 1;
    }

    let codex_credentials_encrypted = sqlx::query_scalar::<_, String>(
        "SELECT value FROM app_security_metadata WHERE key = 'codex_credentials_version'",
    )
    .fetch_optional(&mut *tx)
    .await?
    .as_deref()
        == Some("1");
    let accounts = sqlx::query("SELECT id, password, access_token, refresh_token, id_token, web_session_cookie, web_session_device_id, tempmail_token FROM codex_accounts")
        .fetch_all(&mut *tx)
        .await?;
    for row in accounts {
        let id: String = row.try_get("id")?;
        let fields = [
            ("password", row.try_get::<Option<String>, _>("password")?),
            (
                "access_token",
                row.try_get::<Option<String>, _>("access_token")?,
            ),
            (
                "refresh_token",
                row.try_get::<Option<String>, _>("refresh_token")?,
            ),
            ("id_token", row.try_get::<Option<String>, _>("id_token")?),
            (
                "web_session_cookie",
                row.try_get::<Option<String>, _>("web_session_cookie")?,
            ),
            (
                "web_session_device_id",
                row.try_get::<Option<String>, _>("web_session_device_id")?,
            ),
            (
                "tempmail_token",
                row.try_get::<Option<String>, _>("tempmail_token")?,
            ),
        ];
        for (field, value) in fields {
            if let Some(value) = value {
                // A failed legacy read must not wrap an already-current
                // ciphertext as plaintext, or silently accept corrupted data.
                let encrypted = match reencrypt_value(&value, legacy, current) {
                    Ok(encrypted) => encrypted,
                    Err(_) if !codex_credentials_encrypted => current.encrypt(&value)?,
                    Err(error) => return Err(error),
                };
                sqlx::query(&format!(
                    "UPDATE codex_accounts SET {field} = ? WHERE id = ?"
                ))
                .bind(encrypted)
                .bind(&id)
                .execute(&mut *tx)
                .await?;
                migrated += 1;
            }
        }
    }

    sqlx::query(
        "INSERT INTO app_security_metadata (key, value) VALUES ('encryption_key_version', '2') \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO app_security_metadata (key, value) VALUES ('codex_credentials_version', '1') \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(migrated)
}

pub async fn encryption_key_migration_complete(pool: &SqlitePool) -> anyhow::Result<bool> {
    let value = sqlx::query_scalar::<_, String>(
        "SELECT value FROM app_security_metadata WHERE key = 'encryption_key_version'",
    )
    .fetch_optional(pool)
    .await?;
    Ok(value.as_deref() == Some("2"))
}

pub async fn encrypt_codex_credentials(
    pool: &SqlitePool,
    encryption: &EncryptionService,
) -> anyhow::Result<u64> {
    let completed = sqlx::query_scalar::<_, String>(
        "SELECT value FROM app_security_metadata WHERE key = 'codex_credentials_version'",
    )
    .fetch_optional(pool)
    .await?;
    if completed.as_deref() == Some("1") {
        return Ok(0);
    }
    let mut tx = pool.begin().await?;
    let rows = sqlx::query("SELECT id, password, access_token, refresh_token, id_token, web_session_cookie, web_session_device_id, tempmail_token FROM codex_accounts")
        .fetch_all(&mut *tx).await?;
    let mut migrated = 0;
    for row in rows {
        let id: String = row.try_get("id")?;
        let fields = [
            ("password", row.try_get::<Option<String>, _>("password")?),
            (
                "access_token",
                row.try_get::<Option<String>, _>("access_token")?,
            ),
            (
                "refresh_token",
                row.try_get::<Option<String>, _>("refresh_token")?,
            ),
            ("id_token", row.try_get::<Option<String>, _>("id_token")?),
            (
                "web_session_cookie",
                row.try_get::<Option<String>, _>("web_session_cookie")?,
            ),
            (
                "web_session_device_id",
                row.try_get::<Option<String>, _>("web_session_device_id")?,
            ),
            (
                "tempmail_token",
                row.try_get::<Option<String>, _>("tempmail_token")?,
            ),
        ];
        for (field, value) in fields {
            let Some(value) = value else { continue };
            if encryption.decrypt(&value).is_ok() {
                continue;
            }
            let encrypted = encryption.encrypt(&value)?;
            sqlx::query(&format!(
                "UPDATE codex_accounts SET {field} = ? WHERE id = ?"
            ))
            .bind(encrypted)
            .bind(&id)
            .execute(&mut *tx)
            .await?;
            migrated += 1;
        }
    }
    sqlx::query("INSERT INTO app_security_metadata (key, value) VALUES ('codex_credentials_version', '1') ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(migrated)
}
