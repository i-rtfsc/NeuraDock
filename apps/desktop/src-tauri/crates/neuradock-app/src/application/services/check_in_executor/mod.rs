use anyhow::{Context, Result};
use log::info;
use std::sync::Arc;
use tracing::instrument;

use neuradock_domain::waf_cookies::WafCookiesRepository;
use neuradock_domain::{account::AccountRepository, check_in::Provider, shared::AccountId};
use neuradock_infrastructure::http::{CheckInResult, HttpClient, SetCookieResult, UserInfo};

use crate::application::services::user_info_service::UserInfoService;
use crate::application::services::waf_cookie_manager::WafCookieManager;

mod balance;
mod execution;
mod types;
mod validation;
mod waf_handler;

pub use types::AccountCheckInResult;

/// Check-in executor service
pub struct CheckInExecutor {
    http_client: HttpClient,
    waf_manager: WafCookieManager,
    account_repo: Arc<dyn AccountRepository>,
}

impl CheckInExecutor {
    pub fn new(account_repo: Arc<dyn AccountRepository>, headless_browser: bool) -> Result<Self> {
        Self::with_proxy(account_repo, headless_browser, None)
    }

    pub fn with_proxy(
        account_repo: Arc<dyn AccountRepository>,
        headless_browser: bool,
        proxy_url: Option<String>,
    ) -> Result<Self> {
        let http_client = HttpClient::with_proxy(proxy_url.clone())?;
        let waf_manager = WafCookieManager::new(headless_browser, proxy_url);

        Ok(Self {
            http_client,
            waf_manager,
            account_repo,
        })
    }

    /// Set WAF cookies repository for caching
    pub fn with_waf_cookies_repo(mut self, repo: Arc<dyn WafCookiesRepository>) -> Self {
        self.waf_manager = self.waf_manager.with_cookies_repo(repo);
        self
    }

    /// Create UserInfoService from current executor state
    fn create_user_info_service(&self) -> UserInfoService<'_> {
        UserInfoService::new(&self.http_client, &self.waf_manager)
    }

    /// Execute check-in for a single account
    #[instrument(skip(self, provider), fields(account_id = %account_id, provider_id = %provider.id()))]
    pub async fn execute_check_in(
        &self,
        account_id: &str,
        provider: &Provider,
    ) -> Result<AccountCheckInResult> {
        let account_id_obj = AccountId::from_string(account_id);

        // 1. Load and validate account
        let mut account =
            validation::load_and_validate_account(&*self.account_repo, &account_id_obj).await?;
        let account_name = account.name().to_string();

        info!("[{}] Starting check-in process", account_name);

        // 2. Validate using domain service
        if let Some(error_result) =
            validation::validate_check_in_eligibility(&account, provider, &account_name)
        {
            return Ok(error_result);
        }

        // 3. Prepare cookies and fetch user info with WAF handling
        let (mut cookies, user_info, info_set_cookies) = self
            .prepare_cookies_and_fetch_user_info(&account, provider, &account_name)
            .await?;

        // Collect all Set-Cookie results for later persistence
        let mut all_set_cookies = info_set_cookies;

        // 4. Execute check-in request
        let (check_in_result, checkin_set_cookies) = self
            .perform_check_in_request(&account, provider, &account_name, &mut cookies)
            .await;
        all_set_cookies.cookies.extend(checkin_set_cookies.cookies);
        if checkin_set_cookies.session_expires_at.is_some() {
            all_set_cookies.session_expires_at = checkin_set_cookies.session_expires_at;
        }

        // 5. Fetch updated balance after successful check-in
        let user_info_service = self.create_user_info_service();
        let (final_user_info, balance_set_cookies) =
            balance::fetch_updated_balance_after_check_in(
                &user_info_service,
                &account,
                provider,
                &account_name,
                &cookies,
                &check_in_result,
                user_info,
            )
            .await;
        all_set_cookies.cookies.extend(balance_set_cookies.cookies);
        if balance_set_cookies.session_expires_at.is_some() {
            all_set_cookies.session_expires_at = balance_set_cookies.session_expires_at;
        }

        // 6. Persist updated cookies and session expiration
        if !all_set_cookies.is_empty() || all_set_cookies.session_expires_at.is_some() {
            self.persist_updated_cookies(&mut account, &all_set_cookies, &account_name)
                .await;
        }

        Ok(AccountCheckInResult {
            account_name,
            success: check_in_result.success,
            message: check_in_result.message,
            user_info: final_user_info,
        })
    }

    /// Fetch balance only (without triggering check-in)
    /// Only calls /api/user/self to get user info
    #[instrument(skip(self, provider), fields(account_id = %account_id, provider_id = %provider.id()))]
    pub async fn fetch_balance_only(
        &self,
        account_id: &str,
        provider: &Provider,
    ) -> Result<UserInfo> {
        let account_id_obj = AccountId::from_string(account_id);

        // Load account
        let mut account = self
            .account_repo
            .find_by_id(&account_id_obj)
            .await
            .context("Failed to load account")?
            .ok_or_else(|| anyhow::anyhow!("Account not found"))?;

        let account_name = account.name().to_string();

        info!(
            "[{}] Fetching balance (query only, no check-in)",
            account_name
        );

        // Prepare cookies
        let cookies = self
            .waf_manager
            .prepare_cookies(&account_name, provider, account.credentials().cookies())
            .await?;

        let user_info_service = self.create_user_info_service();
        let api_user = account.credentials().api_user();

        // Get user info (balance)
        let (user_info, set_cookies) = user_info_service
            .fetch_user_info(&account_name, provider, &cookies, api_user)
            .await?;

        // Persist updated cookies if any
        if !set_cookies.is_empty() || set_cookies.session_expires_at.is_some() {
            self.persist_updated_cookies(&mut account, &set_cookies, &account_name)
                .await;
        }

        Ok(user_info)
    }

    // ========== Private helper methods for execute_check_in ==========

    /// Prepare cookies and fetch user info with WAF handling
    async fn prepare_cookies_and_fetch_user_info(
        &self,
        account: &neuradock_domain::account::Account,
        provider: &Provider,
        account_name: &str,
    ) -> Result<(std::collections::HashMap<String, String>, Option<UserInfo>, SetCookieResult)> {
        let user_info_service = self.create_user_info_service();
        let api_user = account.credentials().api_user();

        user_info_service
            .fetch_user_info_with_retry(
                account_name,
                provider,
                account.credentials().cookies(),
                api_user,
            )
            .await
    }

    /// Perform check-in request (page visit or API call) with WAF retry logic
    async fn perform_check_in_request(
        &self,
        account: &neuradock_domain::account::Account,
        provider: &Provider,
        account_name: &str,
        cookies: &mut std::collections::HashMap<String, String>,
    ) -> (CheckInResult, SetCookieResult) {
        let api_user = account.credentials().api_user();

        // Check if provider requires explicit check-in
        let Some(sign_in_url) = provider.sign_in_url() else {
            info!(
                "[{}] Provider {} does not require an explicit check-in request, skipping API call",
                account_name,
                provider.name()
            );
            return (
                CheckInResult {
                    success: true,
                    message: "Provider does not require explicit check-in".to_string(),
                },
                SetCookieResult::default(),
            );
        };

        info!(
            "[{}] Executing check-in request to: {}",
            account_name, sign_in_url
        );

        // Determine if this is a page visit or API call
        // Page URLs typically don't contain /api/
        let is_page_visit = !sign_in_url.contains("/api/");

        if is_page_visit {
            execution::execute_page_visit_check_in(
                &self.http_client,
                account_name,
                &sign_in_url,
                cookies,
            )
            .await
        } else {
            self.execute_api_check_in_with_retry(
                account,
                provider,
                account_name,
                &sign_in_url,
                cookies,
                api_user,
            )
            .await
        }
    }

    /// Execute API check-in with WAF retry logic
    async fn execute_api_check_in_with_retry(
        &self,
        account: &neuradock_domain::account::Account,
        provider: &Provider,
        account_name: &str,
        sign_in_url: &str,
        cookies: &mut std::collections::HashMap<String, String>,
        api_user: &str,
    ) -> (CheckInResult, SetCookieResult) {
        let check_in_call = execution::execute_api_check_in(
            &self.http_client,
            sign_in_url,
            cookies,
            provider.api_user_key(),
            api_user,
            account_name,
        )
        .await;

        match check_in_call {
            Ok((result, set_cookies)) => (result, set_cookies),
            Err(e) if self.waf_manager.is_waf_challenge_error(&e) => {
                waf_handler::retry_check_in_after_waf_refresh(
                    &self.waf_manager,
                    &self.http_client,
                    account,
                    provider,
                    account_name,
                    sign_in_url,
                    cookies,
                    api_user,
                )
                .await
            }
            Err(e) => {
                log::error!("[{}] Check-in request error: {}", account_name, e);
                (
                    execution::create_error_result(&format!("Request failed: {}", e)),
                    SetCookieResult::default(),
                )
            }
        }
    }

    /// Persist updated cookies and session expiration to the account
    async fn persist_updated_cookies(
        &self,
        account: &mut neuradock_domain::account::Account,
        set_cookies: &SetCookieResult,
        account_name: &str,
    ) {
        // Merge updated cookie values into account credentials
        if !set_cookies.is_empty() {
            account.merge_cookies(&set_cookies.cookies);
            log::info!(
                "[{}] Merged {} updated cookie(s) from server response",
                account_name,
                set_cookies.cookies.len(),
            );
        }

        // Update session expiration if present
        if let Some(expires_at) = set_cookies.session_expires_at {
            let session_token = set_cookies
                .cookies
                .get("session")
                .cloned()
                .or_else(|| {
                    account
                        .credentials()
                        .cookies()
                        .get("session")
                        .cloned()
                })
                .unwrap_or_default();
            account.update_session(session_token, expires_at);
            log::info!(
                "[{}] Updated session expiration: {}",
                account_name,
                expires_at.to_rfc3339(),
            );
        }

        // Save to database
        if let Err(e) = self.account_repo.save(account).await {
            log::error!(
                "[{}] Failed to persist updated cookies: {}",
                account_name,
                e
            );
        }
    }
}
