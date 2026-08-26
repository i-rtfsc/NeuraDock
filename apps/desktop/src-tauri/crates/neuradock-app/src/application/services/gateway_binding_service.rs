use async_trait::async_trait;
use neuradock_domain::account::AccountRepository;
use neuradock_domain::check_in::ProviderRepository;
use neuradock_domain::independent_key::{IndependentKeyId, IndependentKeyRepository};
use neuradock_domain::shared::{AccountId, DomainError, ProviderId};
use neuradock_domain::token::TokenId;

use super::{ensure_token_usable, TokenUsabilityError};

#[async_trait]
pub trait CachedTokenReader: Send + Sync {
    async fn get_cached_tokens(
        &self,
        account_id: &AccountId,
    ) -> Result<Vec<neuradock_domain::token::ApiToken>, String>;
}

pub struct GatewayBindingService;

pub struct GatewayBindingConfigInput {
    pub model: Option<String>,
    pub context1m: bool,
    pub impersonate: Option<bool>,
    pub isolated: bool,
}

#[derive(Debug, Clone)]
pub enum GatewayBindingSourceInput {
    AccountToken {
        account_id: String,
        provider_id: String,
        token_id: i64,
        base_url: Option<String>,
    },
    IndependentKey {
        key_id: i64,
    },
}

pub struct NormalizedGatewayBindingConfig {
    pub model: Option<String>,
    pub context1m: bool,
    pub impersonate: bool,
    pub isolated: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ResolvedGatewayUpstream {
    pub id: String,
    pub label: String,
    pub base_url: String,
    pub api_key: String,
    pub model: Option<String>,
    pub impersonate: bool,
}

pub struct GatewayResolverDeps<'a> {
    pub accounts: &'a dyn AccountRepository,
    pub providers: &'a dyn ProviderRepository,
    pub independent_keys: &'a dyn IndependentKeyRepository,
    pub tokens: &'a dyn CachedTokenReader,
}

#[derive(Debug, thiserror::Error)]
pub enum GatewayBindingConfigError {
    #[error("1M context requires selecting a model.")]
    ContextRequiresModel,
}

#[derive(Debug, thiserror::Error)]
pub enum GatewayResolveError {
    #[error("Account not found")]
    AccountNotFound,
    #[error("Token not found")]
    TokenNotFound,
    #[error("Provider not found")]
    ProviderNotFound,
    #[error("The selected provider does not belong to this account.")]
    ProviderMismatch,
    #[error("Key with ID {0} not found")]
    IndependentKeyNotFound(i64),
    #[error("Cannot bind an inactive API key. Please enable it first.")]
    IndependentKeyInactive,
    #[error(transparent)]
    TokenUnusable(#[from] TokenUsabilityError),
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error("{0}")]
    Service(String),
}

impl GatewayBindingService {
    pub fn normalize_config(
        config: GatewayBindingConfigInput,
    ) -> Result<NormalizedGatewayBindingConfig, GatewayBindingConfigError> {
        let model = config.model;
        let context1m = config.context1m;
        if context1m && model.is_none() {
            return Err(GatewayBindingConfigError::ContextRequiresModel);
        }
        let impersonate = config.impersonate.unwrap_or(true);
        Ok(NormalizedGatewayBindingConfig {
            model,
            context1m,
            impersonate,
            isolated: config.isolated,
        })
    }

    pub fn effective_model(model: Option<&str>) -> Option<String> {
        model.map(str::to_string)
    }

    pub async fn resolve_upstream(
        source: &GatewayBindingSourceInput,
        model: Option<&str>,
        impersonate: bool,
        deps: GatewayResolverDeps<'_>,
    ) -> Result<ResolvedGatewayUpstream, GatewayResolveError> {
        match source {
            GatewayBindingSourceInput::AccountToken {
                account_id,
                provider_id,
                token_id,
                base_url,
            } => {
                let account_id = AccountId::from_string(account_id);
                let account = deps
                    .accounts
                    .find_by_id(&account_id)
                    .await?
                    .ok_or(GatewayResolveError::AccountNotFound)?;
                let tokens = deps
                    .tokens
                    .get_cached_tokens(&account_id)
                    .await
                    .map_err(GatewayResolveError::Service)?;
                let token_id = TokenId::new(*token_id);
                let token = tokens
                    .iter()
                    .find(|token| token.id() == &token_id)
                    .ok_or(GatewayResolveError::TokenNotFound)?;
                ensure_token_usable(&account, token)?;

                let provider_id = ProviderId::from_string(provider_id);
                if account.provider_id() != &provider_id {
                    return Err(GatewayResolveError::ProviderMismatch);
                }
                let provider = deps
                    .providers
                    .find_by_id(&provider_id)
                    .await?
                    .ok_or(GatewayResolveError::ProviderNotFound)?;
                let label = format!(
                    "{} · {} · {}",
                    provider.name(),
                    account.name(),
                    token.name()
                );
                Ok(ResolvedGatewayUpstream {
                    id: format!("account:{}:{}", account_id, token_id.value()),
                    label,
                    base_url: base_url
                        .clone()
                        .unwrap_or_else(|| provider.domain().to_string()),
                    api_key: token.key().to_string(),
                    model: model.map(str::to_string),
                    impersonate,
                })
            }
            GatewayBindingSourceInput::IndependentKey { key_id } => {
                let key = deps
                    .independent_keys
                    .find_by_id(&IndependentKeyId::new(*key_id))
                    .await?
                    .ok_or(GatewayResolveError::IndependentKeyNotFound(*key_id))?;
                if !key.is_active() {
                    return Err(GatewayResolveError::IndependentKeyInactive);
                }
                Ok(ResolvedGatewayUpstream {
                    id: format!("independent:{key_id}"),
                    label: key.name().to_string(),
                    base_url: key.base_url().to_string(),
                    api_key: key.api_key().to_string(),
                    model: model.map(str::to_string),
                    impersonate,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use neuradock_domain::account::{Account, Credentials};
    use neuradock_domain::check_in::{Provider, ProviderConfig};
    use neuradock_domain::independent_key::{
        IndependentApiKey, IndependentApiKeyConfig, KeyProviderType,
    };
    use neuradock_domain::token::{ApiToken, ApiTokenConfig, TokenStatus};
    use std::collections::HashMap;

    struct UnusedAccountRepository;

    #[async_trait]
    impl AccountRepository for UnusedAccountRepository {
        async fn save(&self, _: &Account) -> Result<(), DomainError> {
            panic!("account repository must not be used")
        }

        async fn find_by_id(&self, _: &AccountId) -> Result<Option<Account>, DomainError> {
            panic!("account repository must not be used")
        }

        async fn find_by_ids(&self, _: &[AccountId]) -> Result<Vec<Account>, DomainError> {
            panic!("account repository must not be used")
        }

        async fn find_all(&self) -> Result<Vec<Account>, DomainError> {
            panic!("account repository must not be used")
        }

        async fn find_enabled(&self) -> Result<Vec<Account>, DomainError> {
            panic!("account repository must not be used")
        }

        async fn delete(&self, _: &AccountId) -> Result<(), DomainError> {
            panic!("account repository must not be used")
        }
    }

    struct UnusedProviderRepository;

    #[async_trait]
    impl ProviderRepository for UnusedProviderRepository {
        async fn save(&self, _: &Provider) -> Result<(), DomainError> {
            panic!("provider repository must not be used")
        }

        async fn find_by_id(&self, _: &ProviderId) -> Result<Option<Provider>, DomainError> {
            panic!("provider repository must not be used")
        }

        async fn find_all(&self) -> Result<Vec<Provider>, DomainError> {
            panic!("provider repository must not be used")
        }

        async fn delete(&self, _: &ProviderId) -> Result<(), DomainError> {
            panic!("provider repository must not be used")
        }
    }

    struct UnusedTokenReader;

    #[async_trait]
    impl CachedTokenReader for UnusedTokenReader {
        async fn get_cached_tokens(
            &self,
            _: &AccountId,
        ) -> Result<Vec<neuradock_domain::token::ApiToken>, String> {
            panic!("token reader must not be used")
        }
    }

    struct StubIndependentKeyRepository {
        key: Option<IndependentApiKey>,
    }

    struct StubAccountRepository {
        account: Option<Account>,
    }

    #[async_trait]
    impl AccountRepository for StubAccountRepository {
        async fn save(&self, _: &Account) -> Result<(), DomainError> {
            panic!("save must not be used")
        }

        async fn find_by_id(&self, id: &AccountId) -> Result<Option<Account>, DomainError> {
            Ok(self
                .account
                .as_ref()
                .filter(|account| account.id() == id)
                .cloned())
        }

        async fn find_by_ids(&self, _: &[AccountId]) -> Result<Vec<Account>, DomainError> {
            panic!("find_by_ids must not be used")
        }

        async fn find_all(&self) -> Result<Vec<Account>, DomainError> {
            panic!("find_all must not be used")
        }

        async fn find_enabled(&self) -> Result<Vec<Account>, DomainError> {
            panic!("find_enabled must not be used")
        }

        async fn delete(&self, _: &AccountId) -> Result<(), DomainError> {
            panic!("delete must not be used")
        }
    }

    struct StubProviderRepository {
        provider: Option<Provider>,
    }

    #[async_trait]
    impl ProviderRepository for StubProviderRepository {
        async fn save(&self, _: &Provider) -> Result<(), DomainError> {
            panic!("save must not be used")
        }

        async fn find_by_id(&self, id: &ProviderId) -> Result<Option<Provider>, DomainError> {
            Ok(self
                .provider
                .as_ref()
                .filter(|provider| provider.id() == id)
                .cloned())
        }

        async fn find_all(&self) -> Result<Vec<Provider>, DomainError> {
            panic!("find_all must not be used")
        }

        async fn delete(&self, _: &ProviderId) -> Result<(), DomainError> {
            panic!("delete must not be used")
        }
    }

    struct StubTokenReader {
        tokens: Vec<ApiToken>,
    }

    #[async_trait]
    impl CachedTokenReader for StubTokenReader {
        async fn get_cached_tokens(&self, account_id: &AccountId) -> Result<Vec<ApiToken>, String> {
            Ok(self
                .tokens
                .iter()
                .filter(|token| token.account_id() == account_id)
                .cloned()
                .collect())
        }
    }

    #[async_trait]
    impl IndependentKeyRepository for StubIndependentKeyRepository {
        async fn create(&self, _: &IndependentApiKey) -> Result<IndependentKeyId, DomainError> {
            panic!("create must not be used")
        }

        async fn update(&self, _: &IndependentApiKey) -> Result<(), DomainError> {
            panic!("update must not be used")
        }

        async fn delete(&self, _: &IndependentKeyId) -> Result<(), DomainError> {
            panic!("delete must not be used")
        }

        async fn find_by_id(
            &self,
            id: &IndependentKeyId,
        ) -> Result<Option<IndependentApiKey>, DomainError> {
            Ok(self
                .key
                .as_ref()
                .filter(|key| key.id() == Some(id))
                .cloned())
        }

        async fn find_all(&self) -> Result<Vec<IndependentApiKey>, DomainError> {
            panic!("find_all must not be used")
        }

        async fn find_by_provider_type(
            &self,
            _: &KeyProviderType,
        ) -> Result<Vec<IndependentApiKey>, DomainError> {
            panic!("find_by_provider_type must not be used")
        }

        async fn find_active(&self) -> Result<Vec<IndependentApiKey>, DomainError> {
            panic!("find_active must not be used")
        }
    }

    fn independent_key(id: i64, active: bool) -> IndependentApiKey {
        IndependentApiKey::restore(
            IndependentKeyId::new(id),
            IndependentApiKeyConfig {
                name: "Primary key".into(),
                provider_type: KeyProviderType::OpenAI,
                custom_provider_name: None,
                api_key: "sk-secret".into(),
                base_url: Some("https://api.example/v1".into()),
                organization_id: None,
                description: None,
            },
            active,
            Utc::now(),
            Utc::now(),
        )
    }

    async fn resolve_independent(
        key_id: i64,
        key: Option<IndependentApiKey>,
    ) -> Result<ResolvedGatewayUpstream, GatewayResolveError> {
        let accounts = UnusedAccountRepository;
        let providers = UnusedProviderRepository;
        let tokens = UnusedTokenReader;
        let independent_keys = StubIndependentKeyRepository { key };
        GatewayBindingService::resolve_upstream(
            &GatewayBindingSourceInput::IndependentKey { key_id },
            Some("gpt-5"),
            false,
            GatewayResolverDeps {
                accounts: &accounts,
                providers: &providers,
                independent_keys: &independent_keys,
                tokens: &tokens,
            },
        )
        .await
    }

    fn account(provider_id: &str, enabled: bool) -> Account {
        let mut cookies = HashMap::new();
        cookies.insert("session".into(), "test".into());
        Account::builder(
            AccountId::from_string("account-1"),
            "Alice".into(),
            ProviderId::from_string(provider_id),
            Credentials::new(cookies, "user".into()),
        )
        .enabled(enabled)
        .build()
    }

    fn provider(id: &str) -> Provider {
        Provider::restore(
            ProviderId::from_string(id),
            ProviderConfig {
                name: "Relay".into(),
                domain: "https://relay.example/v1/".into(),
                login_path: "/login".into(),
                sign_in_path: None,
                user_info_path: "/api/user/self".into(),
                token_api_path: None,
                models_path: None,
                api_user_key: "new-api-user".into(),
                bypass_method: None,
                supports_check_in: true,
                check_in_bugged: false,
            },
            false,
            Utc::now(),
        )
    }

    fn token(status: TokenStatus) -> ApiToken {
        ApiToken::new(
            TokenId::new(9),
            AccountId::from_string("account-1"),
            ApiTokenConfig {
                name: "Main".into(),
                key: "sk-account".into(),
                status,
                used_quota: 0,
                remain_quota: 100,
                unlimited_quota: false,
                expired_time: None,
                model_limits_enabled: false,
                model_limits: None,
            },
        )
    }

    async fn resolve_account(
        account_provider_id: &str,
        source_provider_id: &str,
        enabled: bool,
        token_status: TokenStatus,
        base_url: Option<String>,
    ) -> Result<ResolvedGatewayUpstream, GatewayResolveError> {
        let accounts = StubAccountRepository {
            account: Some(account(account_provider_id, enabled)),
        };
        let providers = StubProviderRepository {
            provider: Some(provider(source_provider_id)),
        };
        let tokens = StubTokenReader {
            tokens: vec![token(token_status)],
        };
        let independent_keys = StubIndependentKeyRepository { key: None };
        GatewayBindingService::resolve_upstream(
            &GatewayBindingSourceInput::AccountToken {
                account_id: "account-1".into(),
                provider_id: source_provider_id.into(),
                token_id: 9,
                base_url,
            },
            Some("claude-sonnet-4"),
            true,
            GatewayResolverDeps {
                accounts: &accounts,
                providers: &providers,
                independent_keys: &independent_keys,
                tokens: &tokens,
            },
        )
        .await
    }

    #[test]
    fn context_requires_a_model() {
        let result = GatewayBindingService::normalize_config(GatewayBindingConfigInput {
            model: None,
            context1m: true,
            impersonate: None,
            isolated: false,
        });
        assert!(matches!(
            result,
            Err(GatewayBindingConfigError::ContextRequiresModel)
        ));
    }

    #[test]
    fn normalization_applies_defaults_and_preserves_flags() {
        let result = GatewayBindingService::normalize_config(GatewayBindingConfigInput {
            model: Some("gpt-5".into()),
            context1m: true,
            impersonate: None,
            isolated: true,
        })
        .unwrap();
        assert_eq!(result.model.as_deref(), Some("gpt-5"));
        assert!(result.context1m);
        assert!(result.impersonate);
        assert!(result.isolated);
    }

    #[test]
    fn resolved_upstream_preserves_all_runtime_fields() {
        let upstream = ResolvedGatewayUpstream {
            id: "account:acct:1".into(),
            label: "Provider · Alice · Main".into(),
            base_url: "https://relay.example/v1".into(),
            api_key: "sk-secret".into(),
            model: Some("gpt-5".into()),
            impersonate: true,
        };
        assert_eq!(upstream.id, "account:acct:1");
        assert_eq!(upstream.label, "Provider · Alice · Main");
        assert_eq!(upstream.base_url, "https://relay.example/v1");
        assert_eq!(upstream.api_key, "sk-secret");
        assert_eq!(upstream.model.as_deref(), Some("gpt-5"));
        assert!(upstream.impersonate);
    }

    #[tokio::test]
    async fn resolves_active_independent_key() {
        let upstream = resolve_independent(7, Some(independent_key(7, true)))
            .await
            .unwrap();

        assert_eq!(upstream.id, "independent:7");
        assert_eq!(upstream.label, "Primary key");
        assert_eq!(upstream.base_url, "https://api.example/v1");
        assert_eq!(upstream.api_key, "sk-secret");
        assert_eq!(upstream.model.as_deref(), Some("gpt-5"));
        assert!(!upstream.impersonate);
    }

    #[tokio::test]
    async fn rejects_inactive_independent_key() {
        let error = resolve_independent(7, Some(independent_key(7, false)))
            .await
            .unwrap_err();

        assert!(matches!(error, GatewayResolveError::IndependentKeyInactive));
    }

    #[tokio::test]
    async fn reports_missing_independent_key() {
        let error = resolve_independent(7, None).await.unwrap_err();

        assert!(matches!(
            error,
            GatewayResolveError::IndependentKeyNotFound(7)
        ));
    }

    #[tokio::test]
    async fn resolves_active_account_token() {
        let upstream =
            resolve_account("provider-1", "provider-1", true, TokenStatus::Enabled, None)
                .await
                .unwrap();

        assert_eq!(upstream.id, "account:account-1:9");
        assert_eq!(upstream.label, "Relay · Alice · Main");
        assert_eq!(upstream.base_url, "https://relay.example/v1");
        assert_eq!(upstream.api_key, "sk-account");
        assert_eq!(upstream.model.as_deref(), Some("claude-sonnet-4"));
        assert!(upstream.impersonate);
    }

    #[tokio::test]
    async fn rejects_disabled_account() {
        let error = resolve_account(
            "provider-1",
            "provider-1",
            false,
            TokenStatus::Enabled,
            None,
        )
        .await
        .unwrap_err();

        assert!(matches!(
            error,
            GatewayResolveError::TokenUnusable(TokenUsabilityError::AccountDisabled)
        ));
    }

    #[tokio::test]
    async fn rejects_inactive_account_token() {
        let error = resolve_account(
            "provider-1",
            "provider-1",
            true,
            TokenStatus::Disabled,
            None,
        )
        .await
        .unwrap_err();

        assert!(matches!(
            error,
            GatewayResolveError::TokenUnusable(TokenUsabilityError::TokenInactive)
        ));
    }

    #[tokio::test]
    async fn rejects_provider_mismatch() {
        let error = resolve_account("provider-1", "provider-2", true, TokenStatus::Enabled, None)
            .await
            .unwrap_err();

        assert!(matches!(error, GatewayResolveError::ProviderMismatch));
    }
}
