use std::collections::HashMap;
use std::sync::Arc;

use crate::application::commands::account_commands::*;
use crate::application::commands::command_handler::CommandHandler;
use crate::application::commands::handlers::*;
use neuradock_domain::account::{Account, AccountRepository, Credentials};
use neuradock_domain::events::{DomainEvent, EventBus};
use neuradock_domain::shared::{AccountId, DomainError, ProviderId};

// Mock repositories and services for testing

struct MockAccountRepository {
    accounts: tokio::sync::RwLock<HashMap<String, Account>>,
}

impl MockAccountRepository {
    fn new() -> Self {
        Self {
            accounts: tokio::sync::RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl AccountRepository for MockAccountRepository {
    async fn save(&self, account: &Account) -> Result<(), DomainError> {
        let mut accounts = self.accounts.write().await;
        accounts.insert(account.id().as_str().to_string(), account.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &AccountId) -> Result<Option<Account>, DomainError> {
        let accounts = self.accounts.read().await;
        Ok(accounts.get(id.as_str()).cloned())
    }

    async fn find_all(&self) -> Result<Vec<Account>, DomainError> {
        let accounts = self.accounts.read().await;
        Ok(accounts.values().cloned().collect())
    }

    async fn find_enabled(&self) -> Result<Vec<Account>, DomainError> {
        let accounts = self.accounts.read().await;
        Ok(accounts
            .values()
            .filter(|a| a.is_enabled())
            .cloned()
            .collect())
    }

    async fn delete(&self, id: &AccountId) -> Result<(), DomainError> {
        let mut accounts = self.accounts.write().await;
        accounts.remove(id.as_str());
        Ok(())
    }
}

struct MockEventBus {
    event_count: tokio::sync::RwLock<usize>,
}

impl MockEventBus {
    fn new() -> Self {
        Self {
            event_count: tokio::sync::RwLock::new(0),
        }
    }

    async fn get_event_count(&self) -> usize {
        *self.event_count.read().await
    }
}

#[async_trait::async_trait]
impl EventBus for MockEventBus {
    async fn publish(&self, _event: Box<dyn DomainEvent>) -> Result<(), DomainError> {
        let mut count = self.event_count.write().await;
        *count += 1;
        Ok(())
    }
}

// Tests

#[tokio::test]
async fn test_create_account_command_handler() {
    let repo = Arc::new(MockAccountRepository::new());
    let event_bus = Arc::new(MockEventBus::new());
    let handler = CreateAccountCommandHandler::new(repo.clone(), event_bus.clone());

    let mut cookies = HashMap::new();
    cookies.insert("session".to_string(), "test_session_value".to_string());

    let command = CreateAccountCommand {
        name: "Test Account".to_string(),
        provider_id: ProviderId::new().as_str().to_string(),
        cookies,
        api_user: "test@user".to_string(),
        auto_checkin_enabled: Some(true),
        auto_checkin_hour: Some(8),
        auto_checkin_minute: Some(30),
    };

    let result = handler.handle(command).await;
    assert!(result.is_ok());

    let account_id = result.unwrap().account_id;
    let account_id_obj = AccountId::from_string(&account_id);

    // Verify account was saved
    let saved_account = repo.find_by_id(&account_id_obj).await.unwrap();
    assert!(saved_account.is_some());

    let account = saved_account.unwrap();
    assert_eq!(account.name(), "Test Account");
    assert!(account.auto_checkin_enabled());

    // Verify event was published
    let event_count = event_bus.get_event_count().await;
    assert_eq!(event_count, 1);
}

#[tokio::test]
async fn test_create_account_with_empty_name_fails() {
    let repo = Arc::new(MockAccountRepository::new());
    let event_bus = Arc::new(MockEventBus::new());
    let handler = CreateAccountCommandHandler::new(repo, event_bus);

    let command = CreateAccountCommand {
        name: "".to_string(),
        provider_id: ProviderId::new().as_str().to_string(),
        cookies: HashMap::new(),
        api_user: "test@user".to_string(),
        auto_checkin_enabled: Some(false),
        auto_checkin_hour: Some(0),
        auto_checkin_minute: Some(0),
    };

    let result = handler.handle(command).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_account_command_handler() {
    let repo = Arc::new(MockAccountRepository::new());
    let event_bus = Arc::new(MockEventBus::new());

    // Create an account first
    let mut cookies = HashMap::new();
    cookies.insert("session".to_string(), "test_session".to_string());

    let account = Account::new(
        "Original Name".to_string(),
        ProviderId::new(),
        Credentials::new(cookies, "test@user".to_string()),
    )
    .unwrap();
    let account_id = account.id().clone();
    repo.save(&account).await.unwrap();

    // Update the account
    let handler = UpdateAccountCommandHandler::new(repo.clone(), event_bus.clone());
    let command = UpdateAccountCommand {
        account_id: account_id.as_str().to_string(),
        name: Some("Updated Name".to_string()),
        cookies: None,
        api_user: None,
        auto_checkin_enabled: Some(true),
        auto_checkin_hour: Some(10),
        auto_checkin_minute: Some(30),
    };

    let result = handler.handle(command).await;
    assert!(result.is_ok());

    // Verify update
    let updated = repo.find_by_id(&account_id).await.unwrap().unwrap();
    assert_eq!(updated.name(), "Updated Name");
    assert!(updated.auto_checkin_enabled());
    assert_eq!(updated.auto_checkin_hour(), 10);
    assert_eq!(updated.auto_checkin_minute(), 30);

    // Verify event
    let event_count = event_bus.get_event_count().await;
    assert_eq!(event_count, 1);
}

#[tokio::test]
async fn test_delete_account_command_handler() {
    let repo = Arc::new(MockAccountRepository::new());
    let event_bus = Arc::new(MockEventBus::new());

    // Create an account
    let mut cookies = HashMap::new();
    cookies.insert("session".to_string(), "test_session".to_string());

    let account = Account::new(
        "Test Account".to_string(),
        ProviderId::new(),
        Credentials::new(cookies, "test@user".to_string()),
    )
    .unwrap();
    let account_id = account.id().clone();
    repo.save(&account).await.unwrap();

    // Delete it
    let handler = DeleteAccountCommandHandler::new(repo.clone(), event_bus.clone());
    let command = DeleteAccountCommand {
        account_id: account_id.as_str().to_string(),
    };

    let result = handler.handle(command).await;
    assert!(result.is_ok());

    // Verify deletion
    let deleted = repo.find_by_id(&account_id).await.unwrap();
    assert!(deleted.is_none());

    // Verify event
    let event_count = event_bus.get_event_count().await;
    assert_eq!(event_count, 1);
}

#[tokio::test]
async fn test_toggle_account_command_handler() {
    let repo = Arc::new(MockAccountRepository::new());
    let event_bus = Arc::new(MockEventBus::new());

    // Create an enabled account
    let mut cookies = HashMap::new();
    cookies.insert("session".to_string(), "test_session".to_string());

    let account = Account::new(
        "Test Account".to_string(),
        ProviderId::new(),
        Credentials::new(cookies, "test@user".to_string()),
    )
    .unwrap();
    let account_id = account.id().clone();
    repo.save(&account).await.unwrap();

    // Disable it
    let handler = ToggleAccountCommandHandler::new(repo.clone(), event_bus.clone());
    let command = ToggleAccountCommand {
        account_id: account_id.as_str().to_string(),
        enabled: false,
    };

    let result = handler.handle(command).await;
    assert!(result.is_ok());

    // Verify toggle
    let toggled = repo.find_by_id(&account_id).await.unwrap().unwrap();
    assert!(!toggled.is_enabled());

    // Verify event
    let event_count = event_bus.get_event_count().await;
    assert_eq!(event_count, 1);
}

#[tokio::test]
async fn test_update_nonexistent_account_fails() {
    let repo = Arc::new(MockAccountRepository::new());
    let event_bus = Arc::new(MockEventBus::new());
    let handler = UpdateAccountCommandHandler::new(repo, event_bus);

    let command = UpdateAccountCommand {
        account_id: "nonexistent-id".to_string(),
        name: Some("New Name".to_string()),
        cookies: None,
        api_user: None,
        auto_checkin_enabled: None,
        auto_checkin_hour: None,
        auto_checkin_minute: None,
    };

    let result = handler.handle(command).await;
    assert!(result.is_err());
}
