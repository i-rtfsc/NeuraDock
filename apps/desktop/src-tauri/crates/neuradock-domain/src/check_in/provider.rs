use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::shared::ProviderId;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Provider {
    id: ProviderId,
    name: String,
    domain: String,
    login_path: String,
    sign_in_path: Option<String>,
    user_info_path: String,
    api_user_key: String,
    bypass_method: Option<String>,
    is_builtin: bool,
    created_at: DateTime<Utc>,
}

impl Provider {
    pub fn new(
        name: String,
        domain: String,
        login_path: String,
        sign_in_path: Option<String>,
        user_info_path: String,
        api_user_key: String,
        bypass_method: Option<String>,
    ) -> Self {
        Self {
            id: ProviderId::new(),
            name,
            domain,
            login_path,
            sign_in_path,
            user_info_path,
            api_user_key,
            bypass_method,
            is_builtin: false,
            created_at: Utc::now(),
        }
    }

    pub fn builtin(
        id: &str,
        name: String,
        domain: String,
        login_path: String,
        sign_in_path: Option<String>,
        user_info_path: String,
        api_user_key: String,
        bypass_method: Option<String>,
    ) -> Self {
        Self {
            id: ProviderId::from_string(id),
            name,
            domain,
            login_path,
            sign_in_path,
            user_info_path,
            api_user_key,
            bypass_method,
            is_builtin: true,
            created_at: Utc::now(),
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

    pub fn api_user_key(&self) -> &str {
        &self.api_user_key
    }

    pub fn needs_waf_bypass(&self) -> bool {
        self.bypass_method.as_deref() == Some("waf_cookies")
    }

    pub fn is_builtin(&self) -> bool {
        self.is_builtin
    }
}
