mod cache;
mod fetch;
mod token_mapper;
mod waf_handler;

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

use neuradock_domain::account::AccountRepository;
use neuradock_domain::check_in::{Provider, ProviderRepository};
use neuradock_domain::proxy_config::ProxyConfigRepository;
use neuradock_domain::shared::ProviderId;
use neuradock_domain::token::TokenRepository;
use neuradock_domain::waf_cookies::WafCookiesRepository;
use neuradock_infrastructure::http::token::TokenClient;
use neuradock_infrastructure::http::WafBypassService;

pub struct TokenService {
    pub(super) token_repo: Arc<dyn TokenRepository>,
    pub(super) account_repo: Arc<dyn AccountRepository>,
    pub(super) provider_repo: Arc<dyn ProviderRepository>,
    pub(super) proxy_config_repo: Arc<dyn ProxyConfigRepository>,
    pub(super) waf_cookies_repo: Option<Arc<dyn WafCookiesRepository>>,
}

impl TokenService {
    pub fn new(
        token_repo: Arc<dyn TokenRepository>,
        account_repo: Arc<dyn AccountRepository>,
        provider_repo: Arc<dyn ProviderRepository>,
        proxy_config_repo: Arc<dyn ProxyConfigRepository>,
    ) -> Result<Self> {
        Ok(Self {
            token_repo,
            account_repo,
            provider_repo,
            proxy_config_repo,
            waf_cookies_repo: None,
        })
    }

    /// Set WAF cookies repository for caching
    pub fn with_waf_cookies_repo(mut self, repo: Arc<dyn WafCookiesRepository>) -> Self {
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

    pub(super) async fn load_proxy_url(&self) -> Option<String> {
        self.proxy_config_repo
            .get()
            .await
            .ok()
            .and_then(|cfg| cfg.proxy_url())
    }

    pub(super) fn build_token_client(&self, proxy_url: Option<String>) -> Result<TokenClient> {
        TokenClient::with_proxy(proxy_url)
    }

    pub(super) fn build_waf_service(&self, proxy_url: Option<String>) -> WafBypassService {
        WafBypassService::with_proxy(true, proxy_url)
    }
}

#[async_trait::async_trait]
impl crate::application::services::CachedTokenReader for TokenService {
    async fn get_cached_tokens(
        &self,
        account_id: &neuradock_domain::shared::AccountId,
    ) -> Result<Vec<neuradock_domain::token::ApiToken>, String> {
        self.get_cached_tokens(account_id)
            .await
            .map_err(|error| error.to_string())
    }
}
