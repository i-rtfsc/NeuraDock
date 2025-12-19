use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::shared::ProviderId;

/// Configuration for creating a Provider
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub name: String,
    pub domain: String,
    pub login_path: String,
    pub sign_in_path: Option<String>,
    pub user_info_path: String,
    pub token_api_path: Option<String>,
    pub models_path: Option<String>,
    pub api_user_key: String,
    pub bypass_method: Option<String>,
    pub supports_check_in: bool,
    pub check_in_bugged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Provider {
    id: ProviderId,
    name: String,
    domain: String,
    login_path: String,
    sign_in_path: Option<String>,
    user_info_path: String,
    token_api_path: Option<String>,
    models_path: Option<String>,
    api_user_key: String,
    bypass_method: Option<String>,
    supports_check_in: bool,
    check_in_bugged: bool,
    is_builtin: bool,
    created_at: DateTime<Utc>,
}

impl Provider {
    fn normalize_domain(domain: String) -> String {
        domain.trim_end_matches('/').to_string()
    }

    pub fn new(config: ProviderConfig) -> Self {
        Self {
            id: ProviderId::new(),
            name: config.name,
            domain: Self::normalize_domain(config.domain),
            login_path: config.login_path,
            sign_in_path: config.sign_in_path,
            user_info_path: config.user_info_path,
            token_api_path: config.token_api_path,
            models_path: config.models_path,
            api_user_key: config.api_user_key,
            bypass_method: config.bypass_method,
            supports_check_in: config.supports_check_in,
            check_in_bugged: config.check_in_bugged,
            is_builtin: false,
            created_at: Utc::now(),
        }
    }

    pub fn builtin(id: &str, config: ProviderConfig) -> Self {
        Self {
            id: ProviderId::from_string(id),
            name: config.name,
            domain: Self::normalize_domain(config.domain),
            login_path: config.login_path,
            sign_in_path: config.sign_in_path,
            user_info_path: config.user_info_path,
            token_api_path: config.token_api_path,
            models_path: config.models_path,
            api_user_key: config.api_user_key,
            bypass_method: config.bypass_method,
            supports_check_in: config.supports_check_in,
            check_in_bugged: config.check_in_bugged,
            is_builtin: true,
            created_at: Utc::now(),
        }
    }

    /// Restore a provider from persistence
    pub fn restore(
        id: ProviderId,
        config: ProviderConfig,
        is_builtin: bool,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name: config.name,
            domain: Self::normalize_domain(config.domain),
            login_path: config.login_path,
            sign_in_path: config.sign_in_path,
            user_info_path: config.user_info_path,
            token_api_path: config.token_api_path,
            models_path: config.models_path,
            api_user_key: config.api_user_key,
            bypass_method: config.bypass_method,
            supports_check_in: config.supports_check_in,
            check_in_bugged: config.check_in_bugged,
            is_builtin,
            created_at,
        }
    }

    pub fn id(&self) -> &ProviderId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub fn login_path(&self) -> &str {
        &self.login_path
    }

    pub fn sign_in_path(&self) -> Option<&str> {
        self.sign_in_path.as_deref()
    }

    pub fn user_info_path(&self) -> &str {
        &self.user_info_path
    }

    pub fn token_api_path(&self) -> Option<&str> {
        self.token_api_path.as_deref()
    }

    pub fn models_path(&self) -> Option<&str> {
        self.models_path.as_deref()
    }

    pub fn login_url(&self) -> String {
        format!("{}{}", self.domain, self.login_path)
    }

    pub fn sign_in_url(&self) -> Option<String> {
        self.sign_in_path
            .as_ref()
            .map(|p| format!("{}{}", self.domain, p))
    }

    pub fn user_info_url(&self) -> String {
        format!("{}{}", self.domain, self.user_info_path)
    }

    pub fn token_api_url(&self) -> Option<String> {
        self.token_api_path
            .as_ref()
            .map(|p| format!("{}{}", self.domain, p))
    }

    pub fn models_url(&self) -> Option<String> {
        self.models_path
            .as_ref()
            .map(|p| format!("{}{}", self.domain, p))
    }

    pub fn api_user_key(&self) -> &str {
        &self.api_user_key
    }

    pub fn needs_waf_bypass(&self) -> bool {
        self.bypass_method.as_deref() == Some("waf_cookies")
    }

    pub fn supports_check_in(&self) -> bool {
        self.supports_check_in
    }

    pub fn check_in_bugged(&self) -> bool {
        self.check_in_bugged
    }

    pub fn is_builtin(&self) -> bool {
        self.is_builtin
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
