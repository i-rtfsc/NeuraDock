// Domain layer - Pure business logic
// No dependencies on infrastructure or presentation layers

pub mod account;
pub mod balance;
pub mod check_in;
pub mod custom_node;
pub mod events;
pub mod independent_key;
pub mod notification;
pub mod session;
pub mod shared;
pub mod token;

// Re-exports for convenience
pub use events::DomainEvent;
pub use shared::{AccountId, DomainError, ProviderId};
