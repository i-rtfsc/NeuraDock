use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::shared::{DomainError, ProviderId};

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
    pub check_in_day_offset_minutes: i32,
    pub check_in_mode: CheckInMode,
}

/// How the app is allowed to check this provider in.
///
/// `ObserveOnly` is a first-class mode, not a failure: some providers only
/// offer a web check-in, so the app never posts and instead detects the
/// check-in from the next quota increase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CheckInMode {
    /// POST to an API endpoint.
    Api,
    /// Visit a page that performs the check-in as a side effect.
    PageVisit,
    /// Never post; detect the check-in from a balance observation.
    ObserveOnly,
    /// Not available at all (unsupported or temporarily disabled).
    Disabled,
}

impl CheckInMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Api => "api",
            Self::PageVisit => "page_visit",
            Self::ObserveOnly => "observe_only",
            Self::Disabled => "disabled",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "api" => Ok(Self::Api),
            "page_visit" => Ok(Self::PageVisit),
            "observe_only" => Ok(Self::ObserveOnly),
            "disabled" => Ok(Self::Disabled),
            other => Err(DomainError::Validation(format!(
                "Invalid check-in mode: {other}"
            ))),
        }
    }

    /// Whether an attempt may send a request (as opposed to waiting for an
    /// observation to prove the check-in happened).
    pub fn sends_a_request(&self) -> bool {
        matches!(self, Self::Api | Self::PageVisit)
    }

    pub fn is_observe_only(&self) -> bool {
        matches!(self, Self::ObserveOnly)
    }

    /// Infer the mode from provider configuration when none is stored.
    ///
    /// The order matters: a provider with no sign-in endpoint is
    /// observation-only *permanently*, whereas `check_in_bugged` and
    /// `supports_check_in` are temporary availability switches. Keeping the
    /// two apart means re-enabling a bugged provider restores its real mode.
    pub fn infer(
        sign_in_path: Option<&str>,
        supports_check_in: bool,
        check_in_bugged: bool,
    ) -> Self {
        match sign_in_path {
            None => Self::ObserveOnly,
            Some(_) if !supports_check_in || check_in_bugged => Self::Disabled,
            Some(path) if path.contains("/api/") => Self::Api,
            Some(_) => Self::PageVisit,
        }
    }
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
    check_in_day_offset_minutes: i32,
    check_in_mode: CheckInMode,
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
            check_in_day_offset_minutes: config.check_in_day_offset_minutes,
            check_in_mode: config.check_in_mode,
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
            check_in_day_offset_minutes: config.check_in_day_offset_minutes,
            check_in_mode: config.check_in_mode,
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
            check_in_day_offset_minutes: config.check_in_day_offset_minutes,
            check_in_mode: config.check_in_mode,
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

    /// Minutes east of UTC at which this provider resets its daily quota.
    pub fn check_in_day_offset_minutes(&self) -> i32 {
        self.check_in_day_offset_minutes
    }

    pub fn check_in_mode(&self) -> CheckInMode {
        self.check_in_mode
    }

    /// Whether the scheduler may send a request for this provider at all.
    pub fn can_send_check_in_request(&self) -> bool {
        self.supports_check_in && !self.check_in_bugged && self.check_in_mode.sends_a_request()
    }

    pub fn is_builtin(&self) -> bool {
        self.is_builtin
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
