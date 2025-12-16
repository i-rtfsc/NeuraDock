use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;

use crate::persistence::RepositoryErrorMapper;
use crate::security::EncryptionService;
use neuradock_domain::independent_key::{
    IndependentApiKey, IndependentKeyId, IndependentKeyRepository, KeyProviderType,
};
use neuradock_domain::shared::DomainError;

#[derive(FromRow)]
struct IndependentKeyRow {
    id: i64,
    name: String,
    provider_type: String,
    custom_provider_name: Option<String>,
    api_key: String,  // encrypted
    base_url: String,
    organization_id: Option<String>,
    description: Option<String>,
    is_active: i64,
    created_at: String,
    updated_at: String,
}

impl IndependentKeyRow {
    fn to_domain(self, encryption: &EncryptionService) -> Result<IndependentApiKey, DomainError> {
        // Decrypt API key
        let api_key = encryption.decrypt(&self.api_key).map_err(|e| {
            DomainError::DataIntegrity(format!(
                "Failed to decrypt API key for independent key {}: {}",
                self.id, e
            ))
        })?;

        let provider_type = KeyProviderType::from_str(&self.provider_type)
            .ok_or_else(|| {
                DomainError::DataIntegrity(format!("Invalid provider_type: {}", self.provider_type))
            })?;

        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map_err(|e| DomainError::DataIntegrity(format!("Invalid created_at: {}", e)))?
            .with_timezone(&Utc);

        let updated_at = DateTime::parse_from_rfc3339(&self.updated_at)
            .map_err(|e| DomainError::DataIntegrity(format!("Invalid updated_at: {}", e)))?
            .with_timezone(&Utc);

        Ok(IndependentApiKey::restore(
            IndependentKeyId::new(self.id),
            self.name,
            provider_type,
            self.custom_provider_name,
            api_key,
            self.base_url,
            self.organization_id,
            self.description,
            self.is_active != 0,
            created_at,
            updated_at,
        ))
    }
}

pub struct SqliteIndependentKeyRepository {
    pool: Arc<SqlitePool>,
    encryption: Arc<EncryptionService>,
}

impl SqliteIndependentKeyRepository {
    pub fn new(pool: Arc<SqlitePool>, encryption: Arc<EncryptionService>) -> Self {
        Self { pool, encryption }
    }
}

#[async_trait]
impl IndependentKeyRepository for SqliteIndependentKeyRepository {
    async fn create(&self, key: &IndependentApiKey) -> Result<IndependentKeyId, DomainError> {
        let api_key_encrypted = self.encryption.encrypt(key.api_key())
            .map_err(|e| DomainError::DataIntegrity(format!("Failed to encrypt API key: {}", e)))?;
        let created_at = key.created_at().to_rfc3339();
        let updated_at = key.updated_at().to_rfc3339();

        let result = sqlx::query(
            r#"
            INSERT INTO independent_api_keys (
                name, provider_type, custom_provider_name, api_key, base_url,
                organization_id, description, is_active, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(key.name())
        .bind(key.provider_type().as_str())
        .bind(key.custom_provider_name())
        .bind(&api_key_encrypted)
        .bind(key.base_url())
        .bind(key.organization_id())
        .bind(key.description())
        .bind(if key.is_active() { 1 } else { 0 })
        .bind(&created_at)
        .bind(&updated_at)
        .execute(&*self.pool)
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Create independent key"))?;

        Ok(IndependentKeyId::new(result.last_insert_rowid()))
    }

    async fn update(&self, key: &IndependentApiKey) -> Result<(), DomainError> {
        let id = key.id().ok_or_else(|| DomainError::NotFound("Key ID is required for update".to_string()))?;
        let api_key_encrypted = self.encryption.encrypt(key.api_key())
            .map_err(|e| DomainError::DataIntegrity(format!("Failed to encrypt API key: {}", e)))?;
        let updated_at = key.updated_at().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE independent_api_keys
            SET name = ?, api_key = ?, base_url = ?, organization_id = ?,
                description = ?, is_active = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(key.name())
        .bind(&api_key_encrypted)
        .bind(key.base_url())
        .bind(key.organization_id())
        .bind(key.description())
        .bind(if key.is_active() { 1 } else { 0 })
        .bind(&updated_at)
        .bind(id.value())
        .execute(&*self.pool)
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Update independent key"))?;

        Ok(())
    }

    async fn delete(&self, id: &IndependentKeyId) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM independent_api_keys WHERE id = ?")
            .bind(id.value())
            .execute(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Delete independent key"))?;

        Ok(())
    }

    async fn find_by_id(
        &self,
        id: &IndependentKeyId,
    ) -> Result<Option<IndependentApiKey>, DomainError> {
        let row: Option<IndependentKeyRow> = sqlx::query_as(
            r#"
            SELECT id, name, provider_type, custom_provider_name, api_key, base_url,
                   organization_id, description, is_active, created_at, updated_at
            FROM independent_api_keys
            WHERE id = ?
            "#,
        )
        .bind(id.value())
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find independent key by ID"))?;

        match row {
            Some(r) => Ok(Some(r.to_domain(&self.encryption)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> Result<Vec<IndependentApiKey>, DomainError> {
        let rows: Vec<IndependentKeyRow> = sqlx::query_as(
            r#"
            SELECT id, name, provider_type, custom_provider_name, api_key, base_url,
                   organization_id, description, is_active, created_at, updated_at
            FROM independent_api_keys
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find all independent keys"))?;

        rows.into_iter()
            .map(|r| r.to_domain(&self.encryption))
            .collect()
    }

    async fn find_by_provider_type(
        &self,
        provider_type: &KeyProviderType,
    ) -> Result<Vec<IndependentApiKey>, DomainError> {
        let rows: Vec<IndependentKeyRow> = sqlx::query_as(
            r#"
            SELECT id, name, provider_type, custom_provider_name, api_key, base_url,
                   organization_id, description, is_active, created_at, updated_at
            FROM independent_api_keys
            WHERE provider_type = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(provider_type.as_str())
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find independent keys by provider type"))?;

        rows.into_iter()
            .map(|r| r.to_domain(&self.encryption))
            .collect()
    }

    async fn find_active(&self) -> Result<Vec<IndependentApiKey>, DomainError> {
        let rows: Vec<IndependentKeyRow> = sqlx::query_as(
            r#"
            SELECT id, name, provider_type, custom_provider_name, api_key, base_url,
                   organization_id, description, is_active, created_at, updated_at
            FROM independent_api_keys
            WHERE is_active = 1
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find active independent keys"))?;

        rows.into_iter()
            .map(|r| r.to_domain(&self.encryption))
            .collect()
    }
}
