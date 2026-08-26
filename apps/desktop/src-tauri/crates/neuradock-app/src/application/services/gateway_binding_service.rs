use neuradock_domain::account::AccountRepository;
use neuradock_domain::check_in::ProviderRepository;
use neuradock_domain::independent_key::{IndependentKeyId, IndependentKeyRepository};
use neuradock_domain::shared::{AccountId, DomainError, ProviderId};
use neuradock_domain::token::TokenId;

use super::{ensure_token_usable, TokenService, TokenUsabilityError};

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
    pub tokens: &'a TokenService,
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
                    .map_err(|error| GatewayResolveError::Service(error.to_string()))?;
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
}
