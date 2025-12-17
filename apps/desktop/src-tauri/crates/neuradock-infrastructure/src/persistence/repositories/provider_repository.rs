use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;

use neuradock_domain::check_in::{Provider, ProviderRepository};
use neuradock_domain::shared::{DomainError, ProviderId};

use crate::persistence::unit_of_work::RepositoryErrorMapper;
use crate::persistence::SqliteRepositoryBase;

#[derive(Debug, FromRow)]
struct ProviderRow {
    id: String,
    name: String,
    domain: String,
    login_path: String,
    sign_in_path: Option<String>,
    user_info_path: String,
    token_api_path: Option<String>,
    models_path: Option<String>,
    api_user_key: String,
    bypass_method: Option<String>,
    supports_check_in: bool,
    check_in_bugged: bool,
    is_builtin: bool,
    created_at: String,
}

pub struct SqliteProviderRepository {
    base: SqliteRepositoryBase,
}

impl SqliteProviderRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self {
            base: SqliteRepositoryBase::new(pool),
        }
    }

    fn row_to_domain(&self, row: ProviderRow) -> Result<Provider, DomainError> {
        let created_at = DateTime::parse_from_rfc3339(&row.created_at)
            .map_err(|e| DomainError::Validation(format!("Invalid created_at: {}", e)))?
            .with_timezone(&Utc);

        let provider = Provider::with_id(
            ProviderId::from_string(&row.id),
            row.name,
            row.domain,
            row.login_path,
            row.sign_in_path,
            row.user_info_path,
            row.token_api_path,
            row.models_path,
            row.api_user_key,
            row.bypass_method,
            row.supports_check_in,
            row.check_in_bugged,
            row.is_builtin,
            created_at,
        );

        Ok(provider)
    }
}

#[async_trait]
impl ProviderRepository for SqliteProviderRepository {
    async fn save(&self, provider: &Provider) -> Result<(), DomainError> {
        let created_at = provider.created_at().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO providers (
                id, name, domain, login_path, sign_in_path, user_info_path,
                token_api_path, models_path, api_user_key, bypass_method,
                supports_check_in, check_in_bugged,
                is_builtin, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                domain = excluded.domain,
                login_path = excluded.login_path,
                sign_in_path = excluded.sign_in_path,
                user_info_path = excluded.user_info_path,
                token_api_path = excluded.token_api_path,
                models_path = excluded.models_path,
                api_user_key = excluded.api_user_key,
                bypass_method = excluded.bypass_method,
                supports_check_in = excluded.supports_check_in,
                check_in_bugged = excluded.check_in_bugged
            "#,
        )
        .bind(provider.id().as_str())
        .bind(provider.name())
        .bind(provider.domain())
        .bind(provider.login_url().trim_start_matches(provider.domain()))
        .bind(
            provider
                .sign_in_url()
                .as_ref()
                .map(|url| url.trim_start_matches(provider.domain())),
        )
        .bind(
            provider
                .user_info_url()
                .trim_start_matches(provider.domain()),
        )
        .bind(
            provider
                .token_api_url()
                .as_ref()
                .map(|url| url.trim_start_matches(provider.domain())),
        )
        .bind(
            provider
                .models_url()
                .as_ref()
                .map(|url| url.trim_start_matches(provider.domain())),
        )
        .bind(provider.api_user_key())
        .bind(if provider.needs_waf_bypass() {
            Some("waf_cookies")
        } else {
            None
        })
        .bind(provider.supports_check_in())
        .bind(provider.check_in_bugged())
        .bind(provider.is_builtin())
        .bind(created_at)
        .execute(self.base.pool())
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Save provider"))?;

        Ok(())
    }

    async fn find_by_id(&self, id: &ProviderId) -> Result<Option<Provider>, DomainError> {
        let row = sqlx::query_as::<_, ProviderRow>(
            r#"
            SELECT id, name, domain, login_path, sign_in_path, user_info_path,
                   token_api_path, models_path, api_user_key, bypass_method,
                   supports_check_in, check_in_bugged,
                   is_builtin, created_at
            FROM providers
            WHERE id = ?
            "#,
        )
        .bind(id.as_str())
        .fetch_optional(self.base.pool())
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find provider by id"))?;

        match row {
            Some(row) => Ok(Some(self.row_to_domain(row)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> Result<Vec<Provider>, DomainError> {
        let rows = sqlx::query_as::<_, ProviderRow>(
            r#"
            SELECT id, name, domain, login_path, sign_in_path, user_info_path,
                   token_api_path, models_path, api_user_key, bypass_method,
                   supports_check_in, check_in_bugged,
                   is_builtin, created_at
            FROM providers
            ORDER BY is_builtin DESC, created_at ASC
            "#,
        )
        .fetch_all(self.base.pool())
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find all providers"))?;

        rows.into_iter()
            .map(|row| self.row_to_domain(row))
            .collect()
    }

    async fn delete(&self, id: &ProviderId) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM providers WHERE id = ?")
            .bind(id.as_str())
            .execute(self.base.pool())
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Delete provider"))?;

        Ok(())
    }
}
