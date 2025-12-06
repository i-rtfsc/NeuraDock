use anyhow::{Context, Result};
use sqlx::{Row, SqlitePool};
use std::sync::Arc;

use neuradock_domain::custom_node::{
    CustomNodeId, CustomProviderNode, CustomProviderNodeRepository,
};
use neuradock_domain::shared::{DomainError, ProviderId};

use crate::persistence::SqliteRepositoryBase;

pub struct SqliteCustomProviderNodeRepository {
    base: SqliteRepositoryBase,
}

impl SqliteCustomProviderNodeRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self {
            base: SqliteRepositoryBase::new(pool),
        }
    }
}

#[async_trait::async_trait]
impl CustomProviderNodeRepository for SqliteCustomProviderNodeRepository {
    async fn create(&self, node: &CustomProviderNode) -> Result<CustomProviderNode, DomainError> {
        let provider_id = node.provider_id().to_string();
        let name = node.name();
        let base_url = node.base_url();

        let result = sqlx::query(
            r#"
            INSERT INTO custom_provider_nodes (provider_id, name, base_url)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(&provider_id)
        .bind(name)
        .bind(base_url)
        .execute(self.base.pool())
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

    async fn find_by_id(
        &self,
        id: &CustomNodeId,
    ) -> Result<Option<CustomProviderNode>, DomainError> {
        let id_value = id.value();
        let record = sqlx::query(
            r#"
            SELECT id, provider_id, name, base_url, created_at
            FROM custom_provider_nodes
            WHERE id = ?
            "#,
        )
        .bind(id_value)
        .fetch_optional(self.base.pool())
        .await
        .context("Failed to query custom provider node by id")
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(record.map(|r| {
            let id: i64 = r.get("id");
            let provider_id: String = r.get("provider_id");
            let name: String = r.get("name");
            let base_url: String = r.get("base_url");
            let created_at: chrono::NaiveDateTime = r.get("created_at");

            CustomProviderNode::new(
                CustomNodeId::new(id),
                ProviderId::from_string(&provider_id),
                name,
                base_url,
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(created_at, chrono::Utc),
            )
        }))
    }

    async fn find_by_provider(
        &self,
        provider_id: &ProviderId,
    ) -> Result<Vec<CustomProviderNode>, DomainError> {
        let provider_id_str = provider_id.to_string();
        let records = sqlx::query(
            r#"
            SELECT id, provider_id, name, base_url, created_at
            FROM custom_provider_nodes
            WHERE provider_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(&provider_id_str)
        .fetch_all(self.base.pool())
        .await
        .context("Failed to query custom provider nodes by provider")
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(records
            .into_iter()
            .map(|r| {
                let id: i64 = r.get("id");
                let provider_id: String = r.get("provider_id");
                let name: String = r.get("name");
                let base_url: String = r.get("base_url");
                let created_at: chrono::NaiveDateTime = r.get("created_at");

                CustomProviderNode::new(
                    CustomNodeId::new(id),
                    ProviderId::from_string(&provider_id),
                    name,
                    base_url,
                    chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                        created_at,
                        chrono::Utc,
                    ),
                )
            })
            .collect())
    }

    async fn find_all(&self) -> Result<Vec<CustomProviderNode>, DomainError> {
        let records = sqlx::query(
            r#"
            SELECT id, provider_id, name, base_url, created_at
            FROM custom_provider_nodes
            ORDER BY provider_id, created_at ASC
            "#,
        )
        .fetch_all(self.base.pool())
        .await
        .context("Failed to query all custom provider nodes")
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(records
            .into_iter()
            .map(|r| {
                let id: i64 = r.get("id");
                let provider_id: String = r.get("provider_id");
                let name: String = r.get("name");
                let base_url: String = r.get("base_url");
                let created_at: chrono::NaiveDateTime = r.get("created_at");

                CustomProviderNode::new(
                    CustomNodeId::new(id),
                    ProviderId::from_string(&provider_id),
                    name,
                    base_url,
                    chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                        created_at,
                        chrono::Utc,
                    ),
                )
            })
            .collect())
    }

    async fn update(&self, node: &CustomProviderNode) -> Result<(), DomainError> {
        let name = node.name();
        let base_url = node.base_url();
        let id_value = node.id().value();

        sqlx::query(
            r#"
            UPDATE custom_provider_nodes
            SET name = ?, base_url = ?
            WHERE id = ?
            "#,
        )
        .bind(name)
        .bind(base_url)
        .bind(id_value)
        .execute(self.base.pool())
        .await
        .context("Failed to update custom provider node")
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &CustomNodeId) -> Result<(), DomainError> {
        let id_value = id.value();
        sqlx::query(
            r#"
            DELETE FROM custom_provider_nodes
            WHERE id = ?
            "#,
        )
        .bind(id_value)
        .execute(self.base.pool())
        .await
        .context("Failed to delete custom provider node")
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(())
    }
}
