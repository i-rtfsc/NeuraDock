mod cache;
mod fetch;
mod token_mapper;
mod waf_handler;

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;

use neuradock_domain::account::AccountRepository;
use neuradock_domain::check_in::{Provider, ProviderRepository};
use neuradock_domain::shared::ProviderId;
use neuradock_domain::token::TokenRepository;
use neuradock_infrastructure::http::token::TokenClient;
use neuradock_infrastructure::http::WafBypassService;
use neuradock_infrastructure::persistence::repositories::SqliteWafCookiesRepository;

pub struct TokenService {
    pub(super) token_repo: Arc<dyn TokenRepository>,
    pub(super) account_repo: Arc<dyn AccountRepository>,
    pub(super) provider_repo: Arc<dyn ProviderRepository>,
    pub(super) http_client: TokenClient,
    pub(super) waf_service: WafBypassService,
    pub(super) waf_cookies_repo: Option<Arc<SqliteWafCookiesRepository>>,
}

impl TokenService {
    pub fn new(
        token_repo: Arc<dyn TokenRepository>,
        account_repo: Arc<dyn AccountRepository>,
        provider_repo: Arc<dyn ProviderRepository>,
    ) -> Result<Self> {
        Ok(Self {
            token_repo,
            account_repo,
            provider_repo,
            http_client: TokenClient::new()?,
            waf_service: WafBypassService::new(true), // headless by default
            waf_cookies_repo: None,
        })
    }

    /// Set WAF cookies repository for caching
    pub fn with_waf_cookies_repo(mut self, repo: Arc<SqliteWafCookiesRepository>) -> Self {
        self.waf_cookies_repo = Some(repo);
        self
    }

    /// Load provider by ID
    pub(super) async fn load_provider(&self, provider_id: &ProviderId) -> Result<Provider> {
        self.provider_repo
            .find_by_id(provider_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", provider_id))
    }

    /// Build cookie string from HashMap
    pub(super) fn build_cookie_string(&self, cookies: &HashMap<String, String>) -> String {
        cookies
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ")
    }
}
