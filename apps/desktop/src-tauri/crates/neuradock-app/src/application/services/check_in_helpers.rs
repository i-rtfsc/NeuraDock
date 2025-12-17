use anyhow::{Context, Result};
use neuradock_domain::account::{Account, AccountRepository};
use neuradock_domain::shared::AccountId;
use std::collections::HashMap;
use std::sync::Arc;
use neuradock_infrastructure::http::waf_bypass::WafBypassService;

/// Load and validate account from repository
pub async fn load_and_validate_account(
    account_id: &AccountId,
    account_repo: &Arc<dyn AccountRepository>,
) -> Result<Account> {
    let account = account_repo
        .find_by_id(account_id)
        .await
        .context("Failed to load account")?
        .ok_or_else(|| anyhow::anyhow!("Account not found: {}", account_id.as_str()))?;

    if !account.is_enabled() {
        anyhow::bail!("Account is disabled: {}", account.name());
    }

    Ok(account)
}

/// Prepare cookies for check-in, refreshing WAF cookies if needed
pub async fn prepare_cookies(
    account: &Account,
    provider_domain: &str,
    needs_waf_bypass: bool,
    waf_service: &Option<Arc<WafBypassService>>,
) -> Result<HashMap<String, String>> {
    let mut cookies = account.credentials().cookies().clone();

    if needs_waf_bypass {
        if let Some(waf) = waf_service {
            let waf_cookies = waf
                .get_waf_cookies(provider_domain, account.name())
                .await
                .context("Failed to get WAF cookies")?;

            // Merge WAF cookies
            for (key, value) in waf_cookies {
                cookies.insert(key, value);
            }
        }
    }

    Ok(cookies)
}

/// Refresh WAF cookies and retry the operation
pub async fn refresh_waf_cookies_and_retry<F, T>(
    provider_domain: &str,
    account_name: &str,
    waf_service: &Option<Arc<WafBypassService>>,
    retry_fn: F,
) -> Result<T>
where
    F: FnOnce(HashMap<String, String>) -> futures::future::BoxFuture<'static, Result<T>>,
{
    if let Some(waf) = waf_service {
        log::info!(
            "WAF challenge detected for {}, refreshing cookies...",
            account_name
        );

        let fresh_cookies = waf
            .get_waf_cookies(provider_domain, account_name)
            .await
            .context("Failed to refresh WAF cookies")?;

        retry_fn(fresh_cookies).await
    } else {
        anyhow::bail!("WAF challenge detected but no WAF service available")
    }
}
