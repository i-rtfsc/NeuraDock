use sqlx::{Row, SqlitePool};

use super::EncryptionService;

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
        let cookies = current.encrypt(&legacy.decrypt(&cookies)?)?;
        let api_user = current.encrypt(&legacy.decrypt(&api_user)?)?;
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
        let api_key = current.encrypt(&legacy.decrypt(&api_key)?)?;
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
        let secret = current.encrypt(&legacy.decrypt(&secret)?)?;
        sqlx::query("UPDATE developer_tool_connections SET encrypted_secret = ? WHERE id = ?")
            .bind(secret)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        migrated += 1;
    }

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
                let plaintext = legacy.decrypt(&value).unwrap_or(value);
                let encrypted = current.encrypt(&plaintext)?;
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
