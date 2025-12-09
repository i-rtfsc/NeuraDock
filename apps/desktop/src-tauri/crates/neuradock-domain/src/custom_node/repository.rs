use super::{CustomNodeId, CustomProviderNode};
use crate::shared::ProviderId;

#[async_trait::async_trait]
pub trait CustomProviderNodeRepository: Send + Sync {
    async fn create(
        &self,
        node: &CustomProviderNode,
    ) -> Result<CustomProviderNode, crate::shared::DomainError>;
    async fn find_by_id(
        &self,
        id: &CustomNodeId,
    ) -> Result<Option<CustomProviderNode>, crate::shared::DomainError>;
    async fn find_by_provider(
        &self,
        provider_id: &ProviderId,
    ) -> Result<Vec<CustomProviderNode>, crate::shared::DomainError>;
    async fn find_all(&self) -> Result<Vec<CustomProviderNode>, crate::shared::DomainError>;
    async fn update(&self, node: &CustomProviderNode) -> Result<(), crate::shared::DomainError>;
    async fn delete(&self, id: &CustomNodeId) -> Result<(), crate::shared::DomainError>;
}
