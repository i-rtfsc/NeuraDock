use anyhow::{Context, Result};
use chrono::Utc;
use log::{error, info, warn};
use std::sync::Arc;
use tracing::instrument;

use neuradock_domain::balance_history::{
    AttemptCompletion, CheckInOutcome, CheckInSource, CheckInTrigger,
};
use neuradock_domain::check_in::{quota_day_of, CheckInMode, CheckInState};
use neuradock_domain::waf_cookies::WafCookiesRepository;
use neuradock_domain::{account::AccountRepository, check_in::Provider, shared::AccountId};
use neuradock_infrastructure::http::{
    check_in_codes, CheckInResult, HttpClient, SetCookieResult, UserInfo,
};

use crate::application::services::balance_history_service::BalanceHistoryService;
use crate::application::services::user_info_service::UserInfoService;
use crate::application::services::waf_cookie_manager::WafCookieManager;

mod balance;
mod execution;
mod types;
mod validation;
mod waf_handler;

#[cfg(test)]
mod tests;

pub use types::AccountCheckInResult;

/// Check-in executor service
pub struct CheckInExecutor {
    http_client: HttpClient,
    waf_manager: WafCookieManager,
    account_repo: Arc<dyn AccountRepository>,
    /// Records the check-in event itself.
    ///
    /// Recording used to live only in `ExecuteCheckInCommandHandler`, so the
    /// scheduler -- which drives this executor directly -- never wrote
    /// `last_check_in` or a `balance_history` row. Since streaks, the calendar
    /// and the trend chart are all derived from `balance_history`, a user who
    /// relied purely on automation saw an app that looked like it had never
    /// run. Owning it here means every caller records, not just the one that
    /// remembered to.
    balance_history_service: Option<Arc<BalanceHistoryService>>,
}

impl CheckInExecutor {
    /// Build an executor.
    ///
    /// `proxy_url` is mandatory (pass `None` only when the user genuinely has
    /// no proxy configured). There was previously a `new()` shorthand that
    /// defaulted to no proxy, and the scheduler's use of it is what silently
    /// exempted every unattended check-in from the user's proxy settings.
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
            balance_history_service: None,
        })
    }

    /// Set WAF cookies repository for caching
    pub fn with_waf_cookies_repo(mut self, repo: Arc<dyn WafCookiesRepository>) -> Self {
        self.waf_manager = self.waf_manager.with_cookies_repo(repo);
        self
    }

    /// Persist check-in history for every check-in this executor performs.
    pub fn with_balance_history(mut self, service: Arc<BalanceHistoryService>) -> Self {
        self.balance_history_service = Some(service);
        self
    }

    /// Create UserInfoService from current executor state
    fn create_user_info_service(&self) -> UserInfoService<'_> {
        UserInfoService::new(&self.http_client, &self.waf_manager)
    }

    /// Execute check-in for a single account.
    ///
    /// `trigger` decides how much of the guard applies: a scheduled run respects
    /// backoff, while a user who insists on trying now only respects "today is
    /// already done" and "a request is already in flight".
    #[instrument(skip(self, provider), fields(account_id = %account_id, provider_id = %provider.id()))]
    pub async fn execute_check_in(
        &self,
        account_id: &str,
        provider: &Provider,
        trigger: CheckInTrigger,
    ) -> Result<AccountCheckInResult> {
        let account_id_obj = AccountId::from_string(account_id);

        // 1. Load and validate account
        let account =
            validation::load_and_validate_account(&*self.account_repo, &account_id_obj).await?;
        let account_name = account.name().to_string();

        info!("[{}] Starting check-in process", account_name);

        // 2. Validate using domain service (account enabled, provider mode)
        if let Some(error_result) =
            validation::validate_check_in_eligibility(&account, provider, &account_name)
        {
            return Ok(error_result);
        }

        let now = Utc::now();
        let quota_day = quota_day_of(now, provider.check_in_day_offset_minutes());

        // 3. Guard. Answering "is today already satisfied / in flight?" from the
        //    ledger replaces the old elapsed-time heuristic, which counted
        //    balance refreshes as check-ins and did nothing when the configured
        //    interval was zero (the shipped default).
        if let Some(service) = &self.balance_history_service {
            match service.check_in_state(account_id, provider, now).await {
                Ok(CheckInState::Done { .. }) => {
                    info!("[{}] Today is already satisfied", account_name);
                    return Ok(AccountCheckInResult::already_done(account_name));
                }
                Ok(CheckInState::InFlight { .. }) => {
                    return Ok(AccountCheckInResult::skipped(
                        account_name,
                        "A check-in is already in flight for today",
                    ));
                }
                Ok(CheckInState::Backoff { until, .. }) if trigger == CheckInTrigger::Scheduler => {
                    return Ok(AccountCheckInResult::skipped(
                        account_name,
                        format!("Waiting until {} before retrying today", until.to_rfc3339()),
                    ));
                }
                Ok(_) => {}
                Err(e) => {
                    // Fail open: reading the ledger failed, and refusing to try
                    // would silently drop the day.
                    warn!(
                        "[{}] Could not read the check-in state: {}",
                        account_name, e
                    );
                }
            }
        }

        // 4. Open an attempt so a concurrent caller sees a request in flight.
        let attempt_id = match &self.balance_history_service {
            Some(service) => match service
                .start_attempt(account_id, provider, quota_day, trigger, now)
                .await
            {
                Ok(id) => Some(id),
                Err(e) => {
                    warn!(
                        "[{}] Could not open a check-in attempt: {}",
                        account_name, e
                    );
                    None
                }
            },
            None => None,
        };

        // 5. Prepare cookies and fetch user info with WAF handling
        let (mut cookies, user_info, info_set_cookies) = self
            .prepare_cookies_and_fetch_user_info(&account, provider, &account_name)
            .await?;

        let mut all_set_cookies = info_set_cookies;

        // 6. Execute check-in request
        let (check_in_result, checkin_set_cookies) = self
            .perform_check_in_request(&account, provider, &account_name, &mut cookies)
            .await;
        all_set_cookies.cookies.extend(checkin_set_cookies.cookies);
        if checkin_set_cookies.session_expires_at.is_some() {
            all_set_cookies.session_expires_at = checkin_set_cookies.session_expires_at;
        }

        // 7. Commit the confirmed reward before waiting for any balance request.
        //    Agent usage, stale snapshots and refreshes cannot change this
        //    ledger, and a bookkeeping failure must not turn a check-in the
        //    provider accepted into a reported failure.
        let confirmed = check_in_result
            .success
            .then(|| confirmed_reward(provider, &check_in_result));

        if let (Some(service), Some((reward, source))) = (&self.balance_history_service, confirmed)
        {
            if let Err(e) = service
                .save_check_in_reward(account_id, provider, Utc::now(), reward, source, attempt_id)
                .await
            {
                error!(
                    "[{}] Check-in succeeded but recording the reward failed: {}",
                    account_name, e
                );
            }
        }

        // 8. Fetch updated balance after successful check-in
        let user_info_service = self.create_user_info_service();
        let (final_user_info, balance_set_cookies) = balance::fetch_updated_balance_after_check_in(
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

        // 9. Persist updated cookies and session expiration
        let mut account = account;
        if !all_set_cookies.is_empty() || all_set_cookies.session_expires_at.is_some() {
            self.persist_updated_cookies(&mut account, &all_set_cookies, &account_name)
                .await;
        }

        // 10. Record the observation. The scheduler calls this executor
        //     directly, so without it automated runs would leave the calendar
        //     and trend empty.
        self.record_balance_observation(&account, provider, final_user_info.as_ref())
            .await;

        // 11. Close the attempt with what actually happened.
        let outcome = outcome_for(&check_in_result);
        if let (Some(service), Some(attempt_id)) = (&self.balance_history_service, attempt_id) {
            let completion = AttemptCompletion {
                outcome,
                finished_at: Utc::now(),
                error_code: check_in_result.error_code().map(str::to_string),
                http_status: None,
                reward: confirmed.and_then(|(reward, _)| reward),
                source: confirmed.map(|(_, source)| source),
            };
            if let Err(e) = service.finish_attempt(attempt_id, &completion).await {
                error!(
                    "[{}] Could not close the check-in attempt: {}",
                    account_name, e
                );
            }
        }

        Ok(AccountCheckInResult {
            account_name,
            success: check_in_result.success,
            message: check_in_result.message,
            user_info: final_user_info,
            outcome,
        })
    }

    /// Append the post-check-in balance observation to the ledger.
    ///
    /// Best-effort: the check-in already happened, so failing to store a
    /// snapshot must not turn it into a reported failure. The account row's
    /// check-in timestamps are not written here -- they are a cache of the
    /// ledger and are refreshed by the repository when evidence changes.
    async fn record_balance_observation(
        &self,
        account: &neuradock_domain::account::Account,
        provider: &Provider,
        user_info: Option<&UserInfo>,
    ) {
        let (Some(service), Some(user_info)) = (&self.balance_history_service, user_info) else {
            return;
        };

        let balance = crate::application::dtos::BalanceDto {
            current_balance: user_info.current_balance,
            total_consumed: user_info.total_consumed,
            // The scheduler calls this path directly, so it must apply the
            // same correction as manual check-ins and balance refreshes.
            total_quota: account.adjusted_total_quota(user_info.total_quota),
        };
        if let Err(e) = service
            .save_balance_history(account.id().as_str(), provider, &balance)
            .await
        {
            error!(
                "[{}] Failed to save the balance observation: {}",
                account.name(),
                e
            );
        }
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
    ) -> Result<(
        std::collections::HashMap<String, String>,
        Option<UserInfo>,
        SetCookieResult,
    )> {
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

    /// Perform the check-in request the provider's mode calls for.
    ///
    /// The mode is stored configuration, not a guess about the URL: an
    /// observation-only provider must never be posted to, and a provider whose
    /// mode says API but whose path is missing is a configuration error rather
    /// than a silent success.
    async fn perform_check_in_request(
        &self,
        account: &neuradock_domain::account::Account,
        provider: &Provider,
        account_name: &str,
        cookies: &mut std::collections::HashMap<String, String>,
    ) -> (CheckInResult, SetCookieResult) {
        let api_user = account.credentials().api_user();

        let mode = provider.check_in_mode();
        if !mode.sends_a_request() {
            info!(
                "[{}] Provider {} is {:?}; the check-in is detected from the balance instead",
                account_name,
                provider.name(),
                mode
            );
            return (
                CheckInResult::failure(
                    check_in_codes::NOT_SUPPORTED,
                    "This provider has no endpoint that confirms a check-in",
                ),
                SetCookieResult::default(),
            );
        }

        let Some(sign_in_url) = provider.sign_in_url() else {
            warn!(
                "[{}] Provider {} is configured as {:?} but has no sign-in path",
                account_name,
                provider.name(),
                mode
            );
            return (
                CheckInResult::failure(
                    check_in_codes::NOT_SUPPORTED,
                    "This provider has no sign-in path configured",
                ),
                SetCookieResult::default(),
            );
        };

        info!(
            "[{}] Executing check-in request to: {}",
            account_name, sign_in_url
        );

        match mode {
            CheckInMode::Api => {
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
            CheckInMode::PageVisit => {
                execution::execute_page_visit_check_in(
                    &self.http_client,
                    account_name,
                    &sign_in_url,
                    cookies,
                )
                .await
            }
            CheckInMode::ObserveOnly | CheckInMode::Disabled => unreachable!("handled above"),
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
                let code = error_code_for(&e);
                error!("[{}] Check-in request error: {}", account_name, e);
                (
                    execution::create_error_result(code, "The check-in request failed"),
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
                .or_else(|| account.credentials().cookies().get("session").cloned())
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

fn confirmed_reward(provider: &Provider, result: &CheckInResult) -> (Option<f64>, CheckInSource) {
    if let Some(reward) = result.reward {
        return (Some(reward), CheckInSource::ProviderResponse);
    }
    match fixed_reward(provider) {
        Some(reward) => (Some(reward), CheckInSource::ProviderRule),
        None => (None, CheckInSource::ConfirmedUnknown),
    }
}

/// Map a provider result onto the attempt outcome.
///
/// Unknown failures are retryable on purpose: retrying costs one request,
/// whereas calling them permanent silently drops the day.
fn outcome_for(result: &CheckInResult) -> CheckInOutcome {
    if result.success {
        return CheckInOutcome::Success;
    }
    match result.error_code() {
        Some(check_in_codes::ALREADY_CHECKED_IN) => CheckInOutcome::AlreadyConfirmed,
        Some(check_in_codes::HTTP_CLIENT) | Some(check_in_codes::NOT_SUPPORTED) => {
            CheckInOutcome::FailedPermanent
        }
        Some(check_in_codes::WAF_CHALLENGE) => CheckInOutcome::WafBlocked,
        Some(check_in_codes::CANCELLED) => CheckInOutcome::Cancelled,
        _ => CheckInOutcome::FailedRetryable,
    }
}

/// Stable code for a request error raised by the HTTP layer. The layer encodes
/// the class in the message prefix, which is the same contract the WAF retry
/// path already relies on.
fn error_code_for(error: &anyhow::Error) -> &'static str {
    let message = error.to_string();
    if message.starts_with(check_in_codes::WAF_CHALLENGE) {
        check_in_codes::WAF_CHALLENGE
    } else if message.starts_with(check_in_codes::HTTP_CLIENT) {
        check_in_codes::HTTP_CLIENT
    } else if message.starts_with(check_in_codes::HTTP_SERVER) {
        check_in_codes::HTTP_SERVER
    } else if message.contains("timed out") || message.contains("timeout") {
        check_in_codes::TIMEOUT
    } else {
        check_in_codes::NETWORK
    }
}

pub(super) fn fixed_reward(provider: &Provider) -> Option<f64> {
    // These two providers' fixed daily grant was confirmed by the user.
    // Match both identity and host; custom providers are not assumed to pay 25.
    let host = url::Url::parse(provider.domain())
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned));
    if matches!(
        (provider.id().as_str(), host.as_deref()),
        ("anyrouter", Some("anyrouter.top")) | ("agentrouter", Some("agentrouter.org"))
    ) {
        Some(25.0)
    } else {
        None
    }
}
