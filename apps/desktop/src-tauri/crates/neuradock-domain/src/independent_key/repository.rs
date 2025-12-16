use crate::independent_key::{IndependentApiKey, IndependentKeyId, KeyProviderType};
use crate::shared::DomainError;
use async_trait::async_trait;

#[async_trait]
pub trait IndependentKeyRepository: Send + Sync {
    async fn create(&self, key: &IndependentApiKey) -> Result<IndependentKeyId, DomainError>;
    async fn update(&self, key: &IndependentApiKey) -> Result<(), DomainError>;
    async fn delete(&self, id: &IndependentKeyId) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: &IndependentKeyId) -> Result<Option<IndependentApiKey>, DomainError>;
    async fn find_all(&self) -> Result<Vec<IndependentApiKey>, DomainError>;
    async fn find_by_provider_type(&self, provider_type: &KeyProviderType) -> Result<Vec<IndependentApiKey>, DomainError>;
    async fn find_active(&self) -> Result<Vec<IndependentApiKey>, DomainError>;
}
