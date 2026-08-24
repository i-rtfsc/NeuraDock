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

    tx.commit().await?;
    Ok(migrated)
}
