use async_trait::async_trait;

use neuradock_domain::account::Account;
use neuradock_domain::check_in::{CheckInResult, Provider};
use neuradock_domain::plugins::ProviderPlugin;
use neuradock_domain::shared::DomainError;

/// AgentRouter provider plugin
///
/// Implements check-in logic for https://agentrouter.org
pub struct AgentRouterPlugin {
    provider: Provider,
}

impl AgentRouterPlugin {
    pub fn new() -> Self {
        Self {
            provider: Provider::builtin(
                "agentrouter",
                "AgentRouter".to_string(),
                "https://agentrouter.org".to_string(),
                "/login".to_string(),
                None, // No separate sign-in endpoint - check-in happens automatically when querying user info
                "/api/user/self".to_string(),
                Some("/api/token/".to_string()),
                Some("/api/user/models".to_string()),
                "new-api-user".to_string(),
                None, // No WAF bypass needed
            ),
        }
    }

    /// Get the provider configuration
    pub fn provider(&self) -> &Provider {
        &self.provider
    }
}

impl Default for AgentRouterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderPlugin for AgentRouterPlugin {
    fn id(&self) -> &str {
        "agentrouter"
    }

    fn name(&self) -> &str {
        "AgentRouter"
    }

    fn domain(&self) -> &str {
        "https://agentrouter.org"
    }

    async fn check_in(
        &self,
        _account: &Account,
        _headless: bool,
    ) -> Result<CheckInResult, DomainError> {
        // Note: Actual check-in logic is delegated to CheckInExecutor
        // This method is here for the trait contract
        // The real implementation uses command handlers that inject dependencies
        Err(DomainError::Infrastructure(
            "Check-in should be executed via CheckInCommandHandler".to_string(),
        ))
    }

    fn validate_credentials(&self, account: &Account) -> bool {
        // Check if cookies are not empty
        !account.credentials().cookies().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_metadata() {
        let plugin = AgentRouterPlugin::new();

        assert_eq!(plugin.id(), "agentrouter");
        assert_eq!(plugin.name(), "AgentRouter");
        assert_eq!(plugin.domain(), "https://agentrouter.org");

        let metadata = plugin.metadata();
        assert_eq!(metadata.id, "agentrouter");
        assert_eq!(metadata.name, "AgentRouter");
        assert_eq!(metadata.domain, "https://agentrouter.org");
    }

    #[test]
    fn test_provider_configuration() {
        let plugin = AgentRouterPlugin::new();
        let provider = plugin.provider();

        assert_eq!(provider.name(), "AgentRouter");
        assert_eq!(provider.domain(), "https://agentrouter.org");
        assert_eq!(provider.login_url(), "https://agentrouter.org/login");
        assert_eq!(
            provider.user_info_url(),
            "https://agentrouter.org/api/user/self"
        );
        assert!(!provider.needs_waf_bypass());
    }
}
