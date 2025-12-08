use async_trait::async_trait;
use neuradock_domain::shared::DomainError;

/// Command trait - marker trait for all commands
pub trait Command: Send + Sync {}

/// Command handler trait
#[async_trait]
pub trait CommandHandler<C: Command>: Send + Sync {
    type Result;

    async fn handle(&self, command: C) -> Result<Self::Result, DomainError>;
}
