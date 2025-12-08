use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::{PluginInstance, PluginMetadata, ProviderPlugin};
use crate::shared::DomainError;

/// Plugin registry
///
/// Central registry for managing provider plugins.
/// Supports registration, discovery, and retrieval of plugins.
pub struct PluginRegistry {
    plugins: Arc<RwLock<HashMap<String, PluginInstance>>>,
}

impl PluginRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a plugin
    ///
    /// # Arguments
    /// * `plugin` - The plugin instance to register
    ///
    /// # Returns
    /// Ok if registered successfully, Err if plugin ID already exists
    pub async fn register(&self, plugin: PluginInstance) -> Result<(), DomainError> {
        let id = plugin.id().to_string();
        let mut plugins = self.plugins.write().await;

        if plugins.contains_key(&id) {
            return Err(DomainError::Validation(format!(
                "Plugin '{}' is already registered",
                id
            )));
        }

        info!("🔌 Registering plugin: {} ({})", plugin.name(), id);
        plugins.insert(id, plugin);
        Ok(())
    }

    /// Get a plugin by ID
    ///
    /// # Arguments
    /// * `id` - The plugin ID
    ///
    /// # Returns
    /// Some(plugin) if found, None otherwise
    pub async fn get(&self, id: &str) -> Option<PluginInstance> {
        let plugins = self.plugins.read().await;
        plugins.get(id).cloned()
    }

    /// Get all registered plugins
    pub async fn get_all(&self) -> Vec<PluginInstance> {
        let plugins = self.plugins.read().await;
        plugins.values().cloned().collect()
    }

    /// Get all plugin metadata
    pub async fn get_all_metadata(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().await;
        plugins.values().map(|p| p.metadata()).collect()
    }

    /// Check if a plugin is registered
    pub async fn contains(&self, id: &str) -> bool {
        let plugins = self.plugins.read().await;
        plugins.contains_key(id)
    }

    /// Get the number of registered plugins
    pub async fn count(&self) -> usize {
        let plugins = self.plugins.read().await;
        plugins.len()
    }

    /// Unregister a plugin
    ///
    /// # Arguments
    /// * `id` - The plugin ID to remove
    ///
    /// # Returns
    /// true if removed, false if not found
    pub async fn unregister(&self, id: &str) -> bool {
        let mut plugins = self.plugins.write().await;
        plugins.remove(id).is_some()
    }

    /// Clear all plugins
    pub async fn clear(&self) {
        let mut plugins = self.plugins.write().await;
        plugins.clear();
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::Account;
    use crate::check_in::CheckInResult;
    use async_trait::async_trait;

    struct MockPlugin {
        id: String,
        name: String,
    }

    #[async_trait]
    impl ProviderPlugin for MockPlugin {
        fn id(&self) -> &str {
            &self.id
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn domain(&self) -> &str {
            "https://example.com"
        }

        async fn check_in(
            &self,
            _account: &Account,
            _headless: bool,
        ) -> Result<CheckInResult, DomainError> {
            Ok(CheckInResult {
                success: true,
                balance: None,
                message: Some("Mock check-in".to_string()),
            })
        }

        fn validate_credentials(&self, _account: &Account) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn test_register_plugin() {
        let registry = PluginRegistry::new();
        let plugin = Arc::new(MockPlugin {
            id: "test".to_string(),
            name: "Test Plugin".to_string(),
        }) as PluginInstance;

        registry.register(plugin).await.unwrap();
        assert_eq!(registry.count().await, 1);
        assert!(registry.contains("test").await);
    }

    #[tokio::test]
    async fn test_register_duplicate_fails() {
        let registry = PluginRegistry::new();
        let plugin1 = Arc::new(MockPlugin {
            id: "test".to_string(),
            name: "Test Plugin 1".to_string(),
        }) as PluginInstance;
        let plugin2 = Arc::new(MockPlugin {
            id: "test".to_string(),
            name: "Test Plugin 2".to_string(),
        }) as PluginInstance;

        registry.register(plugin1).await.unwrap();
        let result = registry.register(plugin2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_plugin() {
        let registry = PluginRegistry::new();
        let plugin = Arc::new(MockPlugin {
            id: "test".to_string(),
            name: "Test Plugin".to_string(),
        }) as PluginInstance;

        registry.register(plugin).await.unwrap();

        let retrieved = registry.get("test").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id(), "test");
    }

    #[tokio::test]
    async fn test_get_all_plugins() {
        let registry = PluginRegistry::new();

        let plugin1 = Arc::new(MockPlugin {
            id: "test1".to_string(),
            name: "Test Plugin 1".to_string(),
        }) as PluginInstance;

        let plugin2 = Arc::new(MockPlugin {
            id: "test2".to_string(),
            name: "Test Plugin 2".to_string(),
        }) as PluginInstance;

        registry.register(plugin1).await.unwrap();
        registry.register(plugin2).await.unwrap();

        let all = registry.get_all().await;
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_unregister_plugin() {
        let registry = PluginRegistry::new();
        let plugin = Arc::new(MockPlugin {
            id: "test".to_string(),
            name: "Test Plugin".to_string(),
        }) as PluginInstance;

        registry.register(plugin).await.unwrap();
        assert_eq!(registry.count().await, 1);

        let removed = registry.unregister("test").await;
        assert!(removed);
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_clear_plugins() {
        let registry = PluginRegistry::new();

        let plugin1 = Arc::new(MockPlugin {
            id: "test1".to_string(),
            name: "Test Plugin 1".to_string(),
        }) as PluginInstance;

        let plugin2 = Arc::new(MockPlugin {
            id: "test2".to_string(),
            name: "Test Plugin 2".to_string(),
        }) as PluginInstance;

        registry.register(plugin1).await.unwrap();
        registry.register(plugin2).await.unwrap();
        assert_eq!(registry.count().await, 2);

        registry.clear().await;
        assert_eq!(registry.count().await, 0);
    }
}
