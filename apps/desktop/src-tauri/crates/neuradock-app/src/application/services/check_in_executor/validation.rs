use anyhow::{Context, Result};
use log::warn;

use neuradock_domain::balance_history::CheckInOutcome;
use neuradock_domain::{
    account::{Account, AccountRepository},
    check_in::{CheckInDomainService, Provider},
    shared::AccountId,
};
use neuradock_infrastructure::http::check_in_codes;

use super::types::AccountCheckInResult;

/// Load and validate account exists
pub async fn load_and_validate_account(
    account_repo: &dyn AccountRepository,
    account_id: &AccountId,
) -> Result<Account> {
    account_repo
        .find_by_id(account_id)
        .await
        .context("Failed to load account")?
        .ok_or_else(|| anyhow::anyhow!("Account not found"))
}

/// Validate check-in eligibility using domain service
pub fn validate_check_in_eligibility(
    account: &Account,
    provider: &Provider,
    account_name: &str,
) -> Option<AccountCheckInResult> {
    // Check account eligibility
    if let Err(e) = CheckInDomainService::can_check_in(account) {
        warn!("[{}] Check-in validation failed: {}", account_name, e);
        return Some(AccountCheckInResult {
            account_name: account_name.to_string(),
            success: false,
            message: e.to_string(),
            user_info: None,
            outcome: CheckInOutcome::SkippedGuard,
            error_code: None,
        });
    }

    // Validate provider configuration
    if let Err(e) = CheckInDomainService::validate_provider(provider) {
        log::error!("[{}] Provider validation failed: {}", account_name, e);
        return Some(AccountCheckInResult {
            account_name: account_name.to_string(),
            success: false,
            message: e.to_string(),
            user_info: None,
            // A disabled or observation-only provider will not become available
            // by retrying today.
            outcome: CheckInOutcome::FailedPermanent,
            error_code: Some(check_in_codes::NOT_SUPPORTED.to_string()),
        });
    }

    None
}
