use super::Provider;
use crate::shared::{DomainError, ProviderId};
use async_trait::async_trait;

// The `CheckInJob` aggregate and its repository used to live here. Their table
// was dropped in 20250202000001_drop_unused_tables and nothing ever read it
// again, so the model and its 13 tests were describing a thing that no longer
// existed. Running jobs are now an in-memory concern (see `CheckInJobRegistry`)
// and history comes from `check_in_attempts`.

#[async_trait]
pub trait ProviderRepository: Send + Sync {
    async fn save(&self, provider: &Provider) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: &ProviderId) -> Result<Option<Provider>, DomainError>;
    async fn find_all(&self) -> Result<Vec<Provider>, DomainError>;
    async fn delete(&self, id: &ProviderId) -> Result<(), DomainError>;
}
