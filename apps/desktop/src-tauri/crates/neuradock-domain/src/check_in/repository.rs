use super::{CheckInJob, Provider};
use crate::shared::{AccountId, DomainError, JobId, ProviderId};
use async_trait::async_trait;

#[async_trait]
pub trait CheckInJobRepository: Send + Sync {
    async fn save(&self, job: &CheckInJob) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: &JobId) -> Result<Option<CheckInJob>, DomainError>;
    async fn find_by_account(&self, account_id: &AccountId)
        -> Result<Vec<CheckInJob>, DomainError>;
    async fn find_running(&self) -> Result<Vec<CheckInJob>, DomainError>;
}

#[async_trait]
pub trait ProviderRepository: Send + Sync {
    async fn save(&self, provider: &Provider) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: &ProviderId) -> Result<Option<Provider>, DomainError>;
    async fn find_all(&self) -> Result<Vec<Provider>, DomainError>;
    async fn delete(&self, id: &ProviderId) -> Result<(), DomainError>;
}
