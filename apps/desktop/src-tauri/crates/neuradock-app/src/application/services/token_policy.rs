use neuradock_domain::account::Account;
use neuradock_domain::token::ApiToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TokenUsabilityError {
    #[error("The account is disabled.")]
    AccountDisabled,
    #[error("The token is inactive or expired.")]
    TokenInactive,
}

pub fn ensure_token_usable(account: &Account, token: &ApiToken) -> Result<(), TokenUsabilityError> {
    if !account.is_enabled() {
        return Err(TokenUsabilityError::AccountDisabled);
    }
    if !token.is_active() {
        return Err(TokenUsabilityError::TokenInactive);
    }
    Ok(())
}
