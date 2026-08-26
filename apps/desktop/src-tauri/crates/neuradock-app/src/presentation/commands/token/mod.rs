// Token management commands
mod fetch;
pub use fetch::*;

// Claude Code configuration commands
mod claude;
pub use claude::*;

// Codex configuration commands
mod codex;
pub use codex::*;

// Provider nodes management commands
mod nodes;
pub use nodes::*;

// Provider models management commands
mod models;
pub use models::*;

use crate::application::services::{ensure_token_usable, TokenUsabilityError};
use crate::presentation::error::CommandError;
use neuradock_domain::account::Account;
use neuradock_domain::token::ApiToken;

/// Shared business guard for every path that consumes a relay-account token.
/// UI filtering is only a convenience; commands must remain safe when invoked
/// directly or while restoring persisted gateway bindings.
pub(crate) fn ensure_account_token_usable(
    account: &Account,
    token: &ApiToken,
) -> Result<(), CommandError> {
    ensure_token_usable(account, token).map_err(|error| match error {
        TokenUsabilityError::AccountDisabled => CommandError::validation(
            "Cannot configure a token from a disabled account. Enable the account first.",
        ),
        TokenUsabilityError::TokenInactive => CommandError::validation(
            "Cannot configure a disabled or expired token. Enable the token first.",
        ),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use neuradock_domain::account::Credentials;
    use neuradock_domain::shared::ProviderId;
    use neuradock_domain::token::{ApiTokenConfig, TokenId, TokenStatus};

    fn account_and_token(status: TokenStatus) -> (Account, ApiToken) {
        let mut cookies = HashMap::new();
        cookies.insert("session".to_string(), "test".to_string());
        let account = Account::new(
            "Relay account".to_string(),
            ProviderId::from_string("provider-1"),
            Credentials::new(cookies, "user".to_string()),
        )
        .unwrap();
        let token = ApiToken::new(
            TokenId::new(1),
            account.id().clone(),
            ApiTokenConfig {
                name: "Key".to_string(),
                key: "sk-test".to_string(),
                status,
                used_quota: 0,
                remain_quota: 500_000,
                unlimited_quota: false,
                expired_time: None,
                model_limits_enabled: false,
                model_limits: None,
            },
        );
        (account, token)
    }

    #[test]
    fn active_token_from_enabled_account_is_usable() {
        let (account, token) = account_and_token(TokenStatus::Enabled);
        assert!(ensure_account_token_usable(&account, &token).is_ok());
    }

    #[test]
    fn disabled_account_or_token_is_rejected() {
        let (mut account, active_token) = account_and_token(TokenStatus::Enabled);
        account.toggle(false);
        assert!(ensure_account_token_usable(&account, &active_token).is_err());

        let (account, disabled_token) = account_and_token(TokenStatus::Disabled);
        assert!(ensure_account_token_usable(&account, &disabled_token).is_err());
    }
}
