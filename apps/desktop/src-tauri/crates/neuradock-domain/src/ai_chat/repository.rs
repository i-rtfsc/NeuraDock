use super::aggregate::{AiChatService, AiChatServiceId};
use crate::shared::DomainError;
use async_trait::async_trait;

#[async_trait]
pub trait AiChatServiceRepository: Send + Sync {
    /// Save or update an AI chat service
    async fn save(&self, service: &AiChatService) -> Result<(), DomainError>;

    /// Find a service by its ID
    async fn find_by_id(&self, id: &AiChatServiceId) -> Result<Option<AiChatService>, DomainError>;

    /// Find all services ordered by sort_order
    async fn find_all(&self) -> Result<Vec<AiChatService>, DomainError>;

    /// Find all enabled services ordered by sort_order
    async fn find_enabled(&self) -> Result<Vec<AiChatService>, DomainError>;

    /// Delete a service by its ID (only allowed for non-builtin services)
    async fn delete(&self, id: &AiChatServiceId) -> Result<(), DomainError>;

    /// Check if a service with the given ID exists
    async fn exists(&self, id: &AiChatServiceId) -> Result<bool, DomainError>;

    /// Batch update sort orders
    async fn update_sort_orders(&self, orders: &[(AiChatServiceId, i32)]) -> Result<(), DomainError>;
}
