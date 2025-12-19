use anyhow::{Context, Result};
use neuradock_domain::shared::AccountId;
use neuradock_domain::token::ApiToken;
use neuradock_infrastructure::http::token::FetchTokensRequest;

impl super::TokenService {
    /// Fetch and cache tokens from API
    pub async fn fetch_and_cache_tokens(
        &self,
        account_id: &AccountId,
        force_refresh: bool,
    ) -> Result<Vec<ApiToken>> {
        log::info!(
            "fetch_and_cache_tokens: account_id={}, force_refresh={}",
            account_id,
            force_refresh
        );

        // 1. Load account
        let account = self
            .account_repo
            .find_by_id(account_id)
            .await?
            .context("Account not found")?;

        log::info!(
            "Account found: name={}, provider={}",
            account.name(),
            account.provider_id()
        );

        // 2. Check session validity
        if !account.is_session_valid() {
            log::error!("Account session expired for {}", account_id);
            anyhow::bail!("Account session expired, please re-login");
        }

        let session_token = account
            .session_token()
            .context("No session token available")?;

        log::info!("Session token valid for account {}", account_id);

        // 3. If not forcing refresh, try cache first
        if !force_refresh {
            log::info!("Checking cache for account {}", account_id);
            let cached_tokens = self.token_repo.find_by_account(account_id).await?;
            log::info!("Found {} cached tokens", cached_tokens.len());

            if !cached_tokens.is_empty() {
                // Check if cache is stale (> 1 hour)
                let cache_valid = cached_tokens.iter().all(|t| {
                    let elapsed = chrono::Utc::now() - t.fetched_at();
                    elapsed.num_hours() < 1
                });

                if cache_valid {
                    log::info!("Returning cached tokens for account {}", account_id);
                    return Ok(cached_tokens);
                } else {
                    log::info!("Cache is stale, fetching fresh tokens");
                }
            }
        }

        // 4. Fetch from API
        let provider = self.load_provider(account.provider_id()).await?;
        let provider_id_str = provider.id().as_str().to_string();
        let base_url = provider.domain().trim_end_matches('/').to_string();
        let token_api_path = provider
            .token_api_path()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Provider {} is missing token API endpoint configuration",
                    provider.name()
                )
            })?
            .to_string();
        let api_user_header = provider.api_user_key();
        let api_user_header_opt = if api_user_header.is_empty() {
            None
        } else {
            Some(api_user_header)
        };

        // Build initial cookie string with cached WAF cookies if available
        let mut cookies_map = account.credentials().cookies().clone();

        // Try to get cached WAF cookies first
        if let Some(ref waf_cookies_repo) = self.waf_cookies_repo {
            match waf_cookies_repo.get_valid(&provider_id_str).await {
                Ok(Some(cached_waf)) => {
                    log::info!(
                        "Using cached WAF cookies (expires at {})",
                        cached_waf.expires_at
                    );
                    cookies_map.extend(cached_waf.cookies);
                }
                Ok(None) => {
                    log::info!("No valid cached WAF cookies available");
                }
                Err(e) => {
                    log::warn!("Failed to check cached WAF cookies: {}", e);
                }
            }
        }

        let cookie_string = self.build_cookie_string(&cookies_map);
        let api_user = account.credentials().api_user();
        let api_user_opt = if api_user.is_empty() {
            None
        } else {
            Some(api_user)
        };

        log::info!(
            "Fetching tokens from API: url={}{}, has_api_user={}",
            base_url,
            token_api_path,
            api_user_opt.is_some()
        );

        let response = self
            .http_client
            .fetch_tokens(FetchTokensRequest {
                base_url: &base_url,
                token_api_path: &token_api_path,
                cookie_string: &cookie_string,
                api_user_header: api_user_header_opt,
                api_user: api_user_opt,
                page: 0,
                size: 10,
            })
            .await;

        // Handle WAF challenge
        let response = match response {
            Ok(resp) => resp,
            Err(e) if e.to_string().contains("WAF_CHALLENGE") => {
                log::warn!(
                    "WAF challenge detected, invalidating cache and getting fresh WAF cookies..."
                );

                // Invalidate cached WAF cookies first (they are clearly invalid)
                if let Some(ref waf_cookies_repo) = self.waf_cookies_repo {
                    if let Err(e) = waf_cookies_repo.delete(&provider_id_str).await {
                        log::warn!("Failed to delete cached WAF cookies: {}", e);
                    } else {
                        log::info!("Invalidated cached WAF cookies");
                    }
                }

                // Get fresh WAF cookies via browser bypass
                let waf_cookies = self.get_fresh_waf_cookies(&provider, &account).await?;

                // Merge new WAF cookies with existing cookies
                cookies_map.extend(waf_cookies);
                let updated_cookies = self.build_cookie_string(&cookies_map);

                log::info!(
                    "Retrying with fresh WAF cookies (cookie length: {})",
                    updated_cookies.len()
                );

                // Retry with updated cookies
                self.http_client
                    .fetch_tokens(FetchTokensRequest {
                        base_url: &base_url,
                        token_api_path: &token_api_path,
                        cookie_string: &updated_cookies,
                        api_user_header: api_user_header_opt,
                        api_user: api_user_opt,
                        page: 0,
                        size: 10,
                    })
                    .await?
            }
            Err(e) => return Err(e),
        };

        // 5. Convert to domain objects
        let tokens: Vec<ApiToken> = response
            .data
            .items()
            .iter()
            .cloned()
            .map(|data| self.convert_to_domain(data, account_id.clone()))
            .collect::<Result<Vec<_>>>()?;

        // 6. Delete old tokens and save new ones (to handle deleted tokens on server side)
        if !tokens.is_empty() {
            log::info!(
                "Deleting old tokens for account {} before saving new ones",
                account_id
            );
            self.token_repo.delete_by_account(account_id).await?;
            self.token_repo.save_batch(tokens.clone()).await?;
            log::info!("Cached {} tokens for account {}", tokens.len(), account_id);
        }

        Ok(tokens)
    }
}
