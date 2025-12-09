//! Integration tests for event bus
//! Tests complete publish→subscribe→handle flow with multiple handlers

use async_trait::async_trait;
use chrono::Utc;
use neuradock_domain::events::account_events::AccountCreated;
use neuradock_domain::events::{DynamicEventHandler, EventBus};
use neuradock_domain::shared::{AccountId, DomainError, ProviderId};
use neuradock_infrastructure::events::in_memory_event_bus::InMemoryEventBus;
use std::any::Any;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Test handler that counts processed events
#[derive(Clone)]
struct CountingEventHandler {
    counter: Arc<Mutex<usize>>,
}

impl CountingEventHandler {
    fn new() -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)),
        }
    }

    async fn get_count(&self) -> usize {
        *self.counter.lock().await
    }
}

#[async_trait]
impl DynamicEventHandler for CountingEventHandler {
    async fn handle_dynamic(&self, event: &(dyn Any + Send + Sync)) -> Result<(), DomainError> {
        if event.downcast_ref::<AccountCreated>().is_some() {
            let mut count = self.counter.lock().await;
            *count += 1;
            Ok(())
        } else {
            Err(DomainError::Infrastructure("Wrong event type".to_string()))
        }
    }

    fn event_type_name(&self) -> &'static str {
        std::any::type_name::<AccountCreated>()
    }
}

/// Test handler that collects event data
#[derive(Clone)]
struct CollectingEventHandler {
    account_names: Arc<Mutex<Vec<String>>>,
}

impl CollectingEventHandler {
    fn new() -> Self {
        Self {
            account_names: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn get_names(&self) -> Vec<String> {
        self.account_names.lock().await.clone()
    }
}

#[async_trait]
impl DynamicEventHandler for CollectingEventHandler {
    async fn handle_dynamic(&self, event: &(dyn Any + Send + Sync)) -> Result<(), DomainError> {
        if let Some(account_created) = event.downcast_ref::<AccountCreated>() {
            let mut names = self.account_names.lock().await;
            names.push(account_created.name.clone());
            Ok(())
        } else {
            Err(DomainError::Infrastructure("Wrong event type".to_string()))
        }
    }

    fn event_type_name(&self) -> &'static str {
        std::any::type_name::<AccountCreated>()
    }
}

#[tokio::test]
async fn test_single_handler_receives_event() {
    let bus = InMemoryEventBus::new();
    let handler = CountingEventHandler::new();

    // Subscribe handler
    bus.subscribe::<AccountCreated>(Arc::new(handler.clone()))
        .await
        .unwrap();

    // Publish event
    let event = AccountCreated {
        account_id: AccountId::new(),
        name: "Test Account".to_string(),
        provider_id: ProviderId::new(),
        auto_checkin_enabled: true,
        occurred_at: Utc::now(),
    };

    bus.publish(Box::new(event)).await.unwrap();

    // Wait for async processing
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Verify handler was called
    assert_eq!(handler.get_count().await, 1);
}

#[tokio::test]
async fn test_multiple_handlers_all_receive_event() {
    let bus = InMemoryEventBus::new();
    let counter1 = CountingEventHandler::new();
    let counter2 = CountingEventHandler::new();
    let collector = CollectingEventHandler::new();

    // Subscribe all handlers
    bus.subscribe::<AccountCreated>(Arc::new(counter1.clone()))
        .await
        .unwrap();
    bus.subscribe::<AccountCreated>(Arc::new(counter2.clone()))
        .await
        .unwrap();
    bus.subscribe::<AccountCreated>(Arc::new(collector.clone()))
        .await
        .unwrap();

    // Publish event
    let event = AccountCreated {
        account_id: AccountId::new(),
        name: "Multi Handler Test".to_string(),
        provider_id: ProviderId::new(),
        auto_checkin_enabled: false,
        occurred_at: Utc::now(),
    };

    bus.publish(Box::new(event)).await.unwrap();

    // Wait for async processing
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Verify all handlers were called
    assert_eq!(counter1.get_count().await, 1);
    assert_eq!(counter2.get_count().await, 1);
    assert_eq!(collector.get_names().await, vec!["Multi Handler Test"]);
}

#[tokio::test]
async fn test_multiple_events_to_same_handler() {
    let bus = InMemoryEventBus::new();
    let collector = CollectingEventHandler::new();

    bus.subscribe::<AccountCreated>(Arc::new(collector.clone()))
        .await
        .unwrap();

    // Publish multiple events
    for i in 1..=5 {
        let event = AccountCreated {
            account_id: AccountId::new(),
            name: format!("Account {}", i),
            provider_id: ProviderId::new(),
            auto_checkin_enabled: true,
            occurred_at: Utc::now(),
        };
        bus.publish(Box::new(event)).await.unwrap();
    }

    // Wait for async processing
    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

    // Verify all events were processed
    let names = collector.get_names().await;
    assert_eq!(names.len(), 5);
    for i in 1..=5 {
        assert!(names.contains(&format!("Account {}", i)));
    }
}

#[tokio::test]
async fn test_handler_count_tracking() {
    let bus = InMemoryEventBus::new();

    // Initially no handlers
    assert_eq!(bus.handler_count::<AccountCreated>().await, 0);

    // Subscribe first handler
    let handler1 = CountingEventHandler::new();
    bus.subscribe::<AccountCreated>(Arc::new(handler1))
        .await
        .unwrap();
    assert_eq!(bus.handler_count::<AccountCreated>().await, 1);

    // Subscribe second handler
    let handler2 = CountingEventHandler::new();
    bus.subscribe::<AccountCreated>(Arc::new(handler2))
        .await
        .unwrap();
    assert_eq!(bus.handler_count::<AccountCreated>().await, 2);

    // Subscribe third handler
    let handler3 = CollectingEventHandler::new();
    bus.subscribe::<AccountCreated>(Arc::new(handler3))
        .await
        .unwrap();
    assert_eq!(bus.handler_count::<AccountCreated>().await, 3);
}

#[tokio::test]
async fn test_publish_with_no_subscribers() {
    let bus = InMemoryEventBus::new();

    let event = AccountCreated {
        account_id: AccountId::new(),
        name: "No Subscribers".to_string(),
        provider_id: ProviderId::new(),
        auto_checkin_enabled: true,
        occurred_at: Utc::now(),
    };

    // Should not panic or error
    let result = bus.publish(Box::new(event)).await;
    assert!(result.is_ok());
}
