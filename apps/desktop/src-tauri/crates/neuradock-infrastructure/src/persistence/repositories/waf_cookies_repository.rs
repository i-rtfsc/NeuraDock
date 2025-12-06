use chrono::{DateTime, Duration, Utc};
use sqlx::{FromRow, SqlitePool};
use std::collections::HashMap;
use std::sync::Arc;

use neuradock_domain::shared::DomainError;

use crate::persistence::unit_of_work::RepositoryErrorMapper;
use crate::persistence::SqliteRepositoryBase;

/// WAF cookies cache duration (24 hours)
const WAF_CACHE_HOURS: i64 = 24;

#[derive(Debug, Clone)]
pub struct WafCookies {
    pub provider_id: String,
    pub cookies: HashMap<String, String>,
    pub fetched_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl WafCookies {
    pub fn is_valid(&self) -> bool {
        Utc::now() < self.expires_at
    }
}

#[derive(Debug, FromRow)]
struct WafCookiesRow {
    id: i64,
    provider_id: String,
    cookies: String, // JSON object
    fetched_at: String,
    expires_at: String,
}

pub struct SqliteWafCookiesRepository {
    base: SqliteRepositoryBase,
}

impl SqliteWafCookiesRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self {
            base: SqliteRepositoryBase::new(pool),
        }
    }

    fn row_to_domain(&self, row: WafCookiesRow) -> Result<WafCookies, DomainError> {
        let cookies: HashMap<String, String> = serde_json::from_str(&row.cookies)
            .map_err(|e| DomainError::Validation(format!("Invalid cookies JSON: {}", e)))?;

        let fetched_at = DateTime::parse_from_rfc3339(&row.fetched_at)
            .map_err(|e| DomainError::Validation(format!("Invalid fetched_at: {}", e)))?
            .with_timezone(&Utc);

        let expires_at = DateTime::parse_from_rfc3339(&row.expires_at)
            .map_err(|e| DomainError::Validation(format!("Invalid expires_at: {}", e)))?
            .with_timezone(&Utc);

        Ok(WafCookies {
            provider_id: row.provider_id,
            cookies,
            fetched_at,
            expires_at,
        })
    }

    /// Save or update WAF cookies for a provider
    pub async fn save(&self, provider_id: &str, cookies: &HashMap<String, String>) -> Result<(), DomainError> {
        let cookies_json = serde_json::to_string(cookies)
            .map_err(|e| DomainError::Validation(format!("Failed to serialize cookies: {}", e)))?;

        let now = Utc::now();
        let expires_at = now + Duration::hours(WAF_CACHE_HOURS);

        sqlx::query(
            r#"
            INSERT INTO waf_cookies (provider_id, cookies, fetched_at, expires_at)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(provider_id) DO UPDATE SET
                cookies = excluded.cookies,
                fetched_at = excluded.fetched_at,
                expires_at = excluded.expires_at
            "#,
        )
        .bind(provider_id)
        .bind(cookies_json)
        .bind(now.to_rfc3339())
        .bind(expires_at.to_rfc3339())
        .execute(self.base.pool())
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Save WAF cookies"))?;

        log::info!("WAF cookies saved for provider {}, expires at {}", provider_id, expires_at);

        Ok(())
    }

    /// Get valid (non-expired) WAF cookies for a provider
    pub async fn get_valid(&self, provider_id: &str) -> Result<Option<WafCookies>, DomainError> {
        let row = sqlx::query_as::<_, WafCookiesRow>(
            r#"
            SELECT id, provider_id, cookies, fetched_at, expires_at
            FROM waf_cookies
            WHERE provider_id = ?
            "#,
        )
        .bind(provider_id)
        .fetch_optional(self.base.pool())
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Get WAF cookies"))?;

        match row {
            Some(row) => {
                let waf_cookies = self.row_to_domain(row)?;
                if waf_cookies.is_valid() {
                    log::info!("Using cached WAF cookies for provider {}, expires at {}",
                        provider_id, waf_cookies.expires_at);
                    Ok(Some(waf_cookies))
                } else {
                    log::info!("WAF cookies for provider {} have expired", provider_id);
                    Ok(None)
                }
            }
            None => {
                log::info!("No cached WAF cookies for provider {}", provider_id);
                Ok(None)
            }
        }
    }

    /// Delete WAF cookies for a provider
    pub async fn delete(&self, provider_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM waf_cookies WHERE provider_id = ?")
            .bind(provider_id)
            .execute(self.base.pool())
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Delete WAF cookies"))?;

        Ok(())
    }

    /// Clean up all expired WAF cookies
    pub async fn cleanup_expired(&self) -> Result<u64, DomainError> {
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query("DELETE FROM waf_cookies WHERE expires_at < ?")
            .bind(now)
            .execute(self.base.pool())
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Cleanup expired WAF cookies"))?;

        let deleted = result.rows_affected();
        if deleted > 0 {
            log::info!("Cleaned up {} expired WAF cookies", deleted);
        }

        Ok(deleted)
    }
}
