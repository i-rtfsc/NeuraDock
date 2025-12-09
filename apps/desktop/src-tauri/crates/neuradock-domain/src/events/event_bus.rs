use async_trait::async_trait;
use std::any::Any;

use super::DomainEvent;
use crate::shared::DomainError;

/// Event bus trait for publishing domain events
#[async_trait]
pub trait EventBus: Send + Sync {
    /// Publish a domain event
    async fn publish(&self, event: Box<dyn DomainEvent>) -> Result<(), DomainError>;
}

/// Event handler trait for handling specific event types
#[async_trait]
pub trait EventHandler<E: DomainEvent>: Send + Sync {
    /// Handle a domain event
    async fn handle(&self, event: &E) -> Result<(), DomainError>;
}

/// Type-erased event handler for storage in collections
#[async_trait]
pub trait DynamicEventHandler: Send + Sync {
    /// Handle any domain event (type-erased)
    async fn handle_dynamic(&self, event: &(dyn Any + Send + Sync)) -> Result<(), DomainError>;

    /// Get the type name this handler handles
    fn event_type_name(&self) -> &'static str;
}

/// Wrapper to convert typed EventHandler to DynamicEventHandler
pub struct TypedEventHandlerWrapper<E: DomainEvent + 'static, H: EventHandler<E>> {
    handler: H,
    _phantom: std::marker::PhantomData<E>,
}

impl<E: DomainEvent + 'static, H: EventHandler<E>> TypedEventHandlerWrapper<E, H> {
    pub fn new(handler: H) -> Self {
        Self {
            handler,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<E: DomainEvent + 'static, H: EventHandler<E>> DynamicEventHandler
    for TypedEventHandlerWrapper<E, H>
{
    async fn handle_dynamic(&self, event: &(dyn Any + Send + Sync)) -> Result<(), DomainError> {
        if let Some(typed_event) = event.downcast_ref::<E>() {
            self.handler.handle(typed_event).await
        } else {
            Err(DomainError::Infrastructure(
                "Event type mismatch".to_string(),
            ))
        }
    }

    fn event_type_name(&self) -> &'static str {
        std::any::type_name::<E>()
    }
}
