use serde::{Deserialize, Serialize};
use specta::Type;
use std::any::Any;

pub mod event_bus;
pub mod account_events;

pub use event_bus::{EventBus, EventHandler, DynamicEventHandler, TypedEventHandlerWrapper};

/// Base trait for all domain events
/// All events must be Send + Sync for thread safety
/// Events should also implement Clone for event sourcing scenarios
pub trait DomainEvent: Send + Sync + Any {
    /// Convert to Any for type-safe downcasting
    fn as_any(&self) -> &(dyn Any + Send + Sync);
    
    /// Get the type name of this event for routing/matching
    /// This should return the same value as std::any::type_name::<Self>()
    fn event_type_name(&self) -> &'static str;
}
