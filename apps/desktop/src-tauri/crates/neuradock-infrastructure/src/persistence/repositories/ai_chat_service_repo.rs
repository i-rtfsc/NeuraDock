use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;

use crate::persistence::SqliteRepositoryBase;
use neuradock_domain::ai_chat::{AiChatService, AiChatServiceId, AiChatServiceRepository};
use neuradock_domain::shared::DomainError;

#[derive(FromRow)]
struct AiChatServiceRow {
    id: String,
    name: String,
    url: String,
    icon: Option<String>,
    is_builtin: bool,
    is_enabled: bool,
    sort_order: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl AiChatServiceRow {
    fn into_domain(self) -> AiChatService {
        AiChatService::restore(
            AiChatServiceId::from_string(&self.id),
            self.name,
            self.url,
            self.icon,
            self.is_builtin,
            self.is_enabled,
            self.sort_order,
            self.created_at,
            self.updated_at,
        )
    }
}

pub struct SqliteAiChatServiceRepository {
    base: SqliteRepositoryBase,
}

impl SqliteAiChatServiceRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self {
            base: SqliteRepositoryBase::new(pool),
        }
    }
}

#[async_trait]
impl AiChatServiceRepository for SqliteAiChatServiceRepository {
    async fn save(&self, service: &AiChatService) -> Result<(), DomainError> {
        let query = r#"
            INSERT INTO ai_chat_services (id, name, url, icon, is_builtin, is_enabled, sort_order, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(id) DO UPDATE SET
                name = ?2,
                url = ?3,
                icon = ?4,
                is_builtin = ?5,
                is_enabled = ?6,
                sort_order = ?7,
                updated_at = ?9
        "#;

        self.base
            .execute(
                sqlx::query(query)
                    .bind(service.id().as_str())
                    .bind(service.name())
                    .bind(service.url())
                    .bind(service.icon())
                    .bind(service.is_builtin())
                    .bind(service.is_enabled())
                    .bind(service.sort_order())
                    .bind(service.created_at())
                    .bind(service.updated_at()),
                "Save AI chat service",
            )
            .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: &AiChatServiceId) -> Result<Option<AiChatService>, DomainError> {
        let query = r#"
            SELECT id, name, url, icon, is_builtin, is_enabled, sort_order, created_at, updated_at
            FROM ai_chat_services
            WHERE id = ?1
        "#;

        let row: Option<AiChatServiceRow> = self
            .base
            .fetch_optional(
                sqlx::query_as(query).bind(id.as_str()),
                "Find AI chat service by ID",
            )
            .await?;

        Ok(row.map(|r| r.into_domain()))
    }

    async fn find_all(&self) -> Result<Vec<AiChatService>, DomainError> {
        let query = r#"
            SELECT id, name, url, icon, is_builtin, is_enabled, sort_order, created_at, updated_at
            FROM ai_chat_services
            ORDER BY sort_order ASC, name ASC
        "#;

        let rows: Vec<AiChatServiceRow> = self
            .base
            .fetch_all(sqlx::query_as(query), "Find all AI chat services")
            .await?;

        Ok(rows.into_iter().map(|r| r.into_domain()).collect())
    }

    async fn find_enabled(&self) -> Result<Vec<AiChatService>, DomainError> {
        let query = r#"
            SELECT id, name, url, icon, is_builtin, is_enabled, sort_order, created_at, updated_at
            FROM ai_chat_services
            WHERE is_enabled = 1
            ORDER BY sort_order ASC, name ASC
        "#;

        let rows: Vec<AiChatServiceRow> = self
            .base
            .fetch_all(sqlx::query_as(query), "Find enabled AI chat services")
            .await?;

        Ok(rows.into_iter().map(|r| r.into_domain()).collect())
    }

    async fn delete(&self, id: &AiChatServiceId) -> Result<(), DomainError> {
        // First check if service is built-in
        if let Some(service) = self.find_by_id(id).await? {
            if service.is_builtin() {
                return Err(DomainError::Validation(
                    "Cannot delete built-in service".to_string(),
                ));
            }
        }

        let query = "DELETE FROM ai_chat_services WHERE id = ?1 AND is_builtin = 0";

        self.base
            .execute(
                sqlx::query(query).bind(id.as_str()),
                "Delete AI chat service",
            )
            .await?;

        Ok(())
    }

    async fn exists(&self, id: &AiChatServiceId) -> Result<bool, DomainError> {
        let query = "SELECT COUNT(*) as count FROM ai_chat_services WHERE id = ?1";

        #[derive(FromRow)]
        struct CountRow {
            count: i64,
        }

        let result: CountRow = self
            .base
            .fetch_one(
                sqlx::query_as(query).bind(id.as_str()),
                "Check AI chat service exists",
            )
            .await?;

        Ok(result.count > 0)
    }

    async fn update_sort_orders(
        &self,
        orders: &[(AiChatServiceId, i32)],
    ) -> Result<(), DomainError> {
        let query = "UPDATE ai_chat_services SET sort_order = ?1, updated_at = ?2 WHERE id = ?3";
        let now = Utc::now();

        for (id, order) in orders {
            self.base
                .execute(
                    sqlx::query(query)
                        .bind(order)
                        .bind(now)
                        .bind(id.as_str()),
                    "Update AI chat service sort order",
                )
                .await?;
        }

        Ok(())
    }
}
