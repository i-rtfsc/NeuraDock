// Domain layer - Pure business logic
// No dependencies on infrastructure or presentation layers

pub mod account;
pub mod balance;
pub mod check_in;
pub mod events;
pub mod notification;
pub mod plugins;
pub mod session;
pub mod shared;

// Re-exports for convenience
pub use events::DomainEvent;
pub use shared::{AccountId, DomainError, ProviderId};
