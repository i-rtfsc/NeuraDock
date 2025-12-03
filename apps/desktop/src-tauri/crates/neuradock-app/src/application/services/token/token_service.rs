use anyhow::{Context, Result};
use std::sync::Arc;

use neuradock_domain::account::{Account, AccountRepository};
use neuradock_domain::shared::AccountId;
use neuradock_domain::token::{ApiToken, ModelLimits, TokenId, TokenRepository, TokenStatus};
use neuradock_infrastructure::http::token::{TokenClient, TokenData};
use neuradock_infrastructure::http::WafBypassService;

pub struct TokenService {
    token_repo: Arc<dyn TokenRepository>,
    account_repo: Arc<dyn AccountRepository>,
    http_client: TokenClient,
    waf_service: WafBypassService,
}

impl TokenService {
    pub fn new(
        token_repo: Arc<dyn TokenRepository>,
        account_repo: Arc<dyn AccountRepository>,
    ) -> Result<Self> {
        Ok(Self {
            token_repo,
            account_repo,
            http_client: TokenClient::new()?,
            waf_service: WafBypassService::new(true), // headless by default
        })
    }

    /// Fetch and cache tokens from API
    pub async fn fetch_and_cache_tokens(
        &self,
        account_id: &AccountId,
        force_refresh: bool,
    ) -> Result<Vec<ApiToken>> {
        log::info!("fetch_and_cache_tokens: account_id={}, force_refresh={}", account_id, force_refresh);
        
        // 1. Load account
        let account = self
            .account_repo
            .find_by_id(account_id)
            .await?
            .context("Account not found")?;

        log::info!("Account found: name={}, provider={}", account.name(), account.provider_id());

        // 2. Check session validity
        if !account.is_session_valid() {
            log::error!("Account session expired for {}", account_id);
            anyhow::bail!("Account session expired, please re-login");
        }

        let session_token = account
            .session_token()
            .context("No session token available")?;
            
        log::info!("Session token valid, length: {}", session_token.len());

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
        let (base_url, token_api_path) = self.get_provider_urls(&account)?;
        let cookie_string = account.credentials().cookie_string();
        let api_user = account.credentials().api_user();
        let api_user_opt = if api_user.is_empty() { None } else { Some(api_user) };
        
        log::info!("Fetching tokens from API: url={}{}, has_api_user={}", 
                   base_url, token_api_path, api_user_opt.is_some());

        let response = self
            .http_client
            .fetch_tokens(&base_url, &token_api_path, &cookie_string, api_user_opt, 0, 10)
            .await;

        // Handle WAF challenge
        let response = match response {
            Ok(resp) => resp,
            Err(e) if e.to_string().contains("WAF_CHALLENGE") => {
                log::warn!("WAF challenge detected, attempting to refresh cookies...");
                
                // Try to refresh WAF cookies
                let login_url = format!("{}/console/token", base_url);
                let waf_cookies = self.waf_service
                    .get_waf_cookies(&login_url, &account.name())
                    .await
                    .context("Failed to refresh WAF cookies")?;
                
                log::info!("Successfully refreshed {} WAF cookies", waf_cookies.len());
                
                // Merge new WAF cookies with existing session cookies
                let mut updated_cookies = cookie_string.clone();
                for (name, value) in waf_cookies {
                    // Remove old cookie if exists
                    let cookie_prefix = format!("{}=", name);
                    if let Some(start) = updated_cookies.find(&cookie_prefix) {
                        let end = updated_cookies[start..]
                            .find(';')
                            .map(|i| start + i)
                            .unwrap_or(updated_cookies.len());
                        updated_cookies.replace_range(start..end, "");
                        // Clean up extra semicolons
                        updated_cookies = updated_cookies.replace(";;", ";");
                        if updated_cookies.ends_with(";") {
                            updated_cookies.pop();
                        }
                    }
                    // Add new cookie
                    if !updated_cookies.is_empty() && !updated_cookies.ends_with(";") {
                        updated_cookies.push_str("; ");
                    }
                    updated_cookies.push_str(&format!("{}={}", name, value));
                }
                
                log::info!("Retrying with refreshed cookies (length: {})", updated_cookies.len());
                
                // Retry with updated cookies
                self.http_client
                    .fetch_tokens(&base_url, &token_api_path, &updated_cookies, api_user_opt, 0, 10)
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
            log::info!("Deleting old tokens for account {} before saving new ones", account_id);
            self.token_repo.delete_by_account(account_id).await?;
            self.token_repo.save_batch(tokens.clone()).await?;
            log::info!("Cached {} tokens for account {}", tokens.len(), account_id);
        }

        Ok(tokens)
    }

    /// Get cached tokens only
    pub async fn get_cached_tokens(&self, account_id: &AccountId) -> Result<Vec<ApiToken>> {
        self.token_repo.find_by_account(account_id).await.map_err(Into::into)
    }

    fn get_provider_urls(&self, account: &Account) -> Result<(String, String)> {
        let provider_id = account.provider_id();
        
        let (base_url, token_api_path) = match provider_id.to_string().as_str() {
            "anyrouter" => ("https://anyrouter.top".to_string(), "/api/token/".to_string()),
            "agentrouter" => ("https://agentrouter.org".to_string(), "/api/token/".to_string()),
            _ => anyhow::bail!("Unknown provider: {}", provider_id),
        };

        Ok((base_url, token_api_path))
    }

    fn convert_to_domain(&self, data: TokenData, account_id: AccountId) -> Result<ApiToken> {
        let expired_time = if data.expired_time == -1 {
            None
        } else {
            Some(
                chrono::DateTime::from_timestamp(data.expired_time, 0)
                    .context("Invalid expired_time")?,
            )
        };

        let model_limits = if data.model_limits_enabled {
            Some(self.parse_model_limits(&data.model_limits))
        } else {
            None
        };

        let status = TokenStatus::from_i32(data.status)
            .context(format!("Invalid token status: {}", data.status))?;

        Ok(ApiToken::new(
            TokenId::new(data.id),
            account_id,
            data.name,
            data.key,
            status,
            data.used_quota,
            data.remain_quota,
            data.unlimited_quota,
            expired_time,
            model_limits,
        ))
    }

    fn parse_model_limits(&self, limits_json: &serde_json::Value) -> ModelLimits {
        let mut allowed = Vec::new();
        let denied = Vec::new(); // API doesn't provide denied list

        // API returns model_limits as comma-separated string or JSON object
        if let Some(limits_str) = limits_json.as_str() {
            // Parse comma-separated string: "model1,model2,model3"
            if !limits_str.is_empty() {
                allowed = limits_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        } else if let Some(obj) = limits_json.as_object() {
            // Parse JSON object format (for backwards compatibility)
            for (model, config) in obj {
                if let Some(allow) = config.get("allow").and_then(|v| v.as_bool()) {
                    if allow {
                        allowed.push(model.clone());
                    }
                }
            }
        }

        log::debug!("Parsed model limits: {} allowed models", allowed.len());

        ModelLimits { allowed, denied }
    }
}
