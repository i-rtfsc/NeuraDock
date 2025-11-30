use async_trait::async_trait;
use log::{error, info};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use neuradock_domain::events::event_bus::{DynamicEventHandler, EventBus};
use neuradock_domain::events::DomainEvent;
use neuradock_domain::shared::DomainError;

/// In-memory event bus implementation
/// This is a simple implementation that dispatches events synchronously
/// In production, you might want to use a message queue for better scalability
pub struct InMemoryEventBus {
    handlers: Arc<RwLock<HashMap<String, Vec<Arc<dyn DynamicEventHandler>>>>>,
}

impl InMemoryEventBus {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Subscribe a handler to a specific event type
    pub async fn subscribe<E: DomainEvent + 'static>(
        &self,
        handler: Arc<dyn DynamicEventHandler>,
    ) -> Result<(), DomainError> {
        let event_type_name = std::any::type_name::<E>();
        let mut handlers = self.handlers.write().await;
        
        handlers
            .entry(event_type_name.to_string())
            .or_insert_with(Vec::new)
            .push(handler);
        
        info!("Subscribed handler for event type: {}", event_type_name);
        Ok(())
    }

    /// Get the number of handlers for a specific event type
    pub async fn handler_count<E: DomainEvent + 'static>(&self) -> usize {
        let event_type_name = std::any::type_name::<E>();
        let handlers = self.handlers.read().await;
        handlers.get(event_type_name).map_or(0, |h| h.len())
    }
}

impl Default for InMemoryEventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventBus for InMemoryEventBus {
    async fn publish(&self, event: Box<dyn DomainEvent>) -> Result<(), DomainError> {
        let event_any = event.as_any();
        let event_type_name = std::any::type_name_of_val(event_any);
        
        info!("Publishing event: {}", event_type_name);
        
        let handlers = self.handlers.read().await;
        
        if let Some(event_handlers) = handlers.get(event_type_name) {
            info!("Found {} handlers for event type: {}", event_handlers.len(), event_type_name);
            
            // Execute all handlers for this event type
            for handler in event_handlers {
                match handler.handle_dynamic(event_any).await {
                    Ok(_) => {
                        info!("Handler successfully processed event: {}", event_type_name);
                    }
                    Err(e) => {
                        // Log error but continue processing other handlers
                        error!("Handler failed to process event {}: {}", event_type_name, e);
                        // In production, you might want to implement retry logic or dead letter queue
                    }
                }
            }
        } else {
            info!("No handlers registered for event type: {}", event_type_name);
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neuradock_domain::events::account_events::AccountCreated;
    use neuradock_domain::events::event_bus::{EventHandler, TypedEventHandlerWrapper};
    use neuradock_domain::shared::{AccountId, ProviderId};
    use chrono::Utc;

    struct TestEventHandler {
        called: Arc<RwLock<bool>>,
    }

    #[async_trait]
    impl EventHandler<AccountCreated> for TestEventHandler {
        async fn handle(&self, _event: &AccountCreated) -> Result<(), DomainError> {
            let mut called = self.called.write().await;
            *called = true;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_event_bus_publishes_to_handlers() {
        let bus = InMemoryEventBus::new();
        let called = Arc::new(RwLock::new(false));
        
        let handler = TestEventHandler {
            called: called.clone(),
        };
        let wrapper = Arc::new(TypedEventHandlerWrapper::new(handler));
        
        bus.subscribe::<AccountCreated>(wrapper).await.unwrap();
        
        let event = Box::new(AccountCreated {
            account_id: AccountId::new(),
            name: "Test Account".to_string(),
            provider_id: ProviderId::new(),
            auto_checkin_enabled: false,
            occurred_at: Utc::now(),
        });
        
        bus.publish(event).await.unwrap();
        
        let was_called = *called.read().await;
        assert!(was_called, "Handler should have been called");
    }
}
