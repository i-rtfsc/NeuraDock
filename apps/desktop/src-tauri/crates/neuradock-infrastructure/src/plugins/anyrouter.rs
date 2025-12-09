use async_trait::async_trait;

use neuradock_domain::account::Account;
use neuradock_domain::check_in::{CheckInResult, Provider};
use neuradock_domain::plugins::ProviderPlugin;
use neuradock_domain::shared::DomainError;

/// AnyRouter provider plugin
///
/// Implements check-in logic for https://anyrouter.top
pub struct AnyRouterPlugin {
    provider: Provider,
}

impl AnyRouterPlugin {
    pub fn new() -> Self {
        Self {
            provider: Provider::builtin(
                "anyrouter",
                "AnyRouter".to_string(),
                "https://anyrouter.top".to_string(),
                "/login".to_string(),
                Some("/api/user/sign_in".to_string()),
                "/api/user/self".to_string(),
                Some("/api/token/".to_string()),
                Some("/api/user/models".to_string()),
                "new-api-user".to_string(),
                Some("waf_cookies".to_string()),
            ),
        }
    }

    /// Get the provider configuration
    pub fn provider(&self) -> &Provider {
        &self.provider
    }
}

impl Default for AnyRouterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderPlugin for AnyRouterPlugin {
    fn id(&self) -> &str {
        "anyrouter"
    }

    fn name(&self) -> &str {
        "AnyRouter"
    }

    fn domain(&self) -> &str {
        "https://anyrouter.top"
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
        let plugin = AnyRouterPlugin::new();

        assert_eq!(plugin.id(), "anyrouter");
        assert_eq!(plugin.name(), "AnyRouter");
        assert_eq!(plugin.domain(), "https://anyrouter.top");

        let metadata = plugin.metadata();
        assert_eq!(metadata.id, "anyrouter");
        assert_eq!(metadata.name, "AnyRouter");
        assert_eq!(metadata.domain, "https://anyrouter.top");
    }

    #[test]
    fn test_provider_configuration() {
        let plugin = AnyRouterPlugin::new();
        let provider = plugin.provider();

        assert_eq!(provider.name(), "AnyRouter");
        assert_eq!(provider.domain(), "https://anyrouter.top");
        assert_eq!(provider.login_url(), "https://anyrouter.top/login");
        assert_eq!(
            provider.user_info_url(),
            "https://anyrouter.top/api/user/self"
        );
        assert!(provider.needs_waf_bypass());
    }
}
