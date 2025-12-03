use anyhow::{Context, Result};
use sqlx::SqlitePool;
use std::sync::Arc;

use neuradock_domain::shared::{ProviderId, DomainError};
use neuradock_domain::custom_node::{
    CustomNodeId, CustomProviderNode, CustomProviderNodeRepository,
};

pub struct SqliteCustomProviderNodeRepository {
    pool: Arc<SqlitePool>,
}

impl SqliteCustomProviderNodeRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl CustomProviderNodeRepository for SqliteCustomProviderNodeRepository {
    async fn create(&self, node: &CustomProviderNode) -> Result<CustomProviderNode, DomainError> {
        let provider_id = node.provider_id().to_string();
        let name = node.name();
        let base_url = node.base_url();

        let result = sqlx::query!(
            r#"
            INSERT INTO custom_provider_nodes (provider_id, name, base_url)
            VALUES (?, ?, ?)
            "#,
            provider_id,
            name,
            base_url,
        )
        .execute(&*self.pool)
        .await
        .context("Failed to insert custom provider node")
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        let id = CustomNodeId::new(result.last_insert_rowid());

        Ok(CustomProviderNode::new(
            id,
            node.provider_id().clone(),
            node.name().to_string(),
            node.base_url().to_string(),
            node.created_at(),
        ))
    }

    async fn find_by_id(&self, id: &CustomNodeId) -> Result<Option<CustomProviderNode>, DomainError> {
        let id_value = id.value();
        let record = sqlx::query!(
            r#"
            SELECT id, provider_id, name, base_url, created_at
            FROM custom_provider_nodes
            WHERE id = ?
            "#,
            id_value
        )
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to query custom provider node by id")
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(record.map(|r| {
            CustomProviderNode::new(
                CustomNodeId::new(r.id),
                ProviderId::from_string(&r.provider_id),
                r.name,
                r.base_url,
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(r.created_at, chrono::Utc),
            )
        }))
    }

    async fn find_by_provider(&self, provider_id: &ProviderId) -> Result<Vec<CustomProviderNode>, DomainError> {
        let provider_id_str = provider_id.to_string();
        let records = sqlx::query!(
            r#"
            SELECT id, provider_id, name, base_url, created_at
            FROM custom_provider_nodes
            WHERE provider_id = ?
            ORDER BY created_at ASC
            "#,
            provider_id_str
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to query custom provider nodes by provider")
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(records
            .into_iter()
            .filter_map(|r| {
                Some(CustomProviderNode::new(
                    CustomNodeId::new(r.id?),
                    ProviderId::from_string(&r.provider_id),
                    r.name,
                    r.base_url,
                    chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                        r.created_at,
                        chrono::Utc
                    ),
                ))
            })
            .collect())
    }

    async fn find_all(&self) -> Result<Vec<CustomProviderNode>, DomainError> {
        let records = sqlx::query!(
            r#"
            SELECT id, provider_id, name, base_url, created_at
            FROM custom_provider_nodes
            ORDER BY provider_id, created_at ASC
            "#
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to query all custom provider nodes")
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(records
            .into_iter()
            .filter_map(|r| {
                Some(CustomProviderNode::new(
                    CustomNodeId::new(r.id?),
                    ProviderId::from_string(&r.provider_id),
                    r.name,
                    r.base_url,
                    chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                        r.created_at,
                        chrono::Utc
                    ),
                ))
            })
            .collect())
    }

    async fn update(&self, node: &CustomProviderNode) -> Result<(), DomainError> {
        let name = node.name();
        let base_url = node.base_url();
        let id_value = node.id().value();

        sqlx::query!(
            r#"
            UPDATE custom_provider_nodes
            SET name = ?, base_url = ?
            WHERE id = ?
            "#,
            name,
            base_url,
            id_value
        )
        .execute(&*self.pool)
        .await
        .context("Failed to update custom provider node")
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &CustomNodeId) -> Result<(), DomainError> {
        let id_value = id.value();
        sqlx::query!(
            r#"
            DELETE FROM custom_provider_nodes
            WHERE id = ?
            "#,
            id_value
        )
        .execute(&*self.pool)
        .await
        .context("Failed to delete custom provider node")
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(())
    }
}
