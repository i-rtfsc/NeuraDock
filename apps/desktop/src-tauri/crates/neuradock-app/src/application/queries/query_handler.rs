use async_trait::async_trait;
use neuradock_domain::shared::DomainError;

/// Query trait - marker trait for all queries
pub trait Query: Send + Sync {}

/// Query handler trait
#[async_trait]
pub trait QueryHandler<Q: Query>: Send + Sync {
    type Result;
    
    async fn handle(&self, query: Q) -> Result<Self::Result, DomainError>;
}
