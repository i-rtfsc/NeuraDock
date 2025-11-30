pub mod registry;

pub use registry::PluginRegistry;

use async_trait::async_trait;
use std::sync::Arc;

use crate::account::Account;
use crate::check_in::CheckInResult;
use crate::shared::DomainError;

/// Provider plugin trait
/// 
/// Defines the contract for check-in provider plugins.
/// Each provider (AnyRouter, AgentRouter, etc.) implements this trait
/// to provide custom check-in logic.
#[async_trait]
pub trait ProviderPlugin: Send + Sync {
    /// Unique plugin identifier (e.g., "anyrouter", "agentrouter")
    fn id(&self) -> &str;
    
    /// Human-readable plugin name (e.g., "AnyRouter", "AgentRouter")
    fn name(&self) -> &str;
    
    /// Provider domain (e.g., "https://anyrouter.top")
    fn domain(&self) -> &str;
    
    /// Execute check-in for an account
    /// 
    /// # Arguments
    /// * `account` - The account to perform check-in
    /// * `headless` - Whether to run browser in headless mode
    /// 
    /// # Returns
    /// CheckInResult with success status and optional message
    async fn check_in(&self, account: &Account, headless: bool) -> Result<CheckInResult, DomainError>;
    
    /// Validate account credentials
    /// 
    /// Checks if the provided credentials are valid format for this provider.
    /// Does not verify if credentials are correct (that requires actual login).
    fn validate_credentials(&self, account: &Account) -> bool;
    
    /// Get plugin metadata
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: self.id().to_string(),
            name: self.name().to_string(),
            domain: self.domain().to_string(),
            version: "1.0.0".to_string(),
            description: format!("{} check-in provider", self.name()),
        }
    }
}

/// Plugin metadata
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub version: String,
    pub description: String,
}

/// Type alias for plugin instance
pub type PluginInstance = Arc<dyn ProviderPlugin>;
