use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;

use neuradock_domain::shared::DomainError;

use crate::persistence::unit_of_work::RepositoryErrorMapper;
use crate::persistence::SqliteRepositoryBase;

#[derive(Debug, Clone)]
pub struct ProviderModels {
    pub provider_id: String,
    pub models: Vec<String>,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct ProviderModelsRow {
    #[allow(dead_code)]
    id: i64,
    provider_id: String,
    models: String, // JSON array
    fetched_at: String,
}

pub struct SqliteProviderModelsRepository {
    base: SqliteRepositoryBase,
}

impl SqliteProviderModelsRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self {
            base: SqliteRepositoryBase::new(pool),
        }
    }

    fn row_to_domain(&self, row: ProviderModelsRow) -> Result<ProviderModels, DomainError> {
        let models: Vec<String> = serde_json::from_str(&row.models)
            .map_err(|e| DomainError::Validation(format!("Invalid models JSON: {}", e)))?;

        let fetched_at = DateTime::parse_from_rfc3339(&row.fetched_at)
            .map_err(|e| DomainError::Validation(format!("Invalid fetched_at: {}", e)))?
            .with_timezone(&Utc);

        Ok(ProviderModels {
            provider_id: row.provider_id,
            models,
            fetched_at,
        })
    }

    /// Save or update provider models
    pub async fn save(&self, provider_id: &str, models: &[String]) -> Result<(), DomainError> {
        let models_json = serde_json::to_string(models)
            .map_err(|e| DomainError::Validation(format!("Failed to serialize models: {}", e)))?;
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO provider_models (provider_id, models, fetched_at)
            VALUES (?, ?, ?)
            ON CONFLICT(provider_id) DO UPDATE SET
                models = excluded.models,
                fetched_at = excluded.fetched_at
            "#,
        )
        .bind(provider_id)
        .bind(models_json)
        .bind(now)
        .execute(self.base.pool())
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Save provider models"))?;

        Ok(())
    }

    /// Find models by provider ID
    pub async fn find_by_provider(
        &self,
        provider_id: &str,
    ) -> Result<Option<ProviderModels>, DomainError> {
        let row = sqlx::query_as::<_, ProviderModelsRow>(
            r#"
            SELECT id, provider_id, models, fetched_at
            FROM provider_models
            WHERE provider_id = ?
            "#,
        )
        .bind(provider_id)
        .fetch_optional(self.base.pool())
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find provider models"))?;

        match row {
            Some(row) => Ok(Some(self.row_to_domain(row)?)),
            None => Ok(None),
        }
    }

    /// Check if provider models are stale (older than specified hours)
    pub async fn is_stale(
        &self,
        provider_id: &str,
        max_age_hours: i64,
    ) -> Result<bool, DomainError> {
        let models = self.find_by_provider(provider_id).await?;

        match models {
            Some(m) => {
                let age = Utc::now().signed_duration_since(m.fetched_at);
                Ok(age.num_hours() >= max_age_hours)
            }
            None => Ok(true), // No data means it's stale
        }
    }

    /// Delete models for a provider
    pub async fn delete_by_provider(&self, provider_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM provider_models WHERE provider_id = ?")
            .bind(provider_id)
            .execute(self.base.pool())
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Delete provider models"))?;

        Ok(())
    }
}
