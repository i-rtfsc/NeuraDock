use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
pub struct IndependentKeyId(i64);

impl IndependentKeyId {
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    pub fn value(&self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum KeyProviderType {
    OpenAI,
    Anthropic,
    Custom,
}

impl KeyProviderType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(Self::OpenAI),
            "anthropic" => Some(Self::Anthropic),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::Custom => "custom",
        }
    }

    pub fn default_base_url(&self) -> &str {
        match self {
            Self::OpenAI => "https://api.openai.com/v1",
            Self::Anthropic => "https://api.anthropic.com/v1",
            Self::Custom => "",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::OpenAI => "OpenAI",
            Self::Anthropic => "Anthropic (Claude)",
            Self::Custom => "Custom",
        }
    }
}

/// Configuration for creating an IndependentApiKey
#[derive(Debug, Clone)]
pub struct IndependentApiKeyConfig {
    pub name: String,
    pub provider_type: KeyProviderType,
    pub custom_provider_name: Option<String>,
    pub api_key: String,
    pub base_url: Option<String>,
    pub organization_id: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct IndependentApiKey {
    id: Option<IndependentKeyId>,
    name: String,
    provider_type: KeyProviderType,
    custom_provider_name: Option<String>,
    api_key: String,
    base_url: String,
    organization_id: Option<String>,
    description: Option<String>,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl IndependentApiKey {
    pub fn create(config: IndependentApiKeyConfig) -> Self {
        let final_base_url = config
            .base_url
            .unwrap_or_else(|| config.provider_type.default_base_url().to_string());

        Self {
            id: None,
            name: config.name,
            provider_type: config.provider_type,
            custom_provider_name: config.custom_provider_name,
            api_key: config.api_key,
            base_url: final_base_url,
            organization_id: config.organization_id,
            description: config.description,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Restore an IndependentApiKey from persistence
    pub fn restore(
        id: IndependentKeyId,
        config: IndependentApiKeyConfig,
        is_active: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        let final_base_url = config
            .base_url
            .unwrap_or_else(|| config.provider_type.default_base_url().to_string());

        Self {
            id: Some(id),
            name: config.name,
            provider_type: config.provider_type,
            custom_provider_name: config.custom_provider_name,
            api_key: config.api_key,
            base_url: final_base_url,
            organization_id: config.organization_id,
            description: config.description,
            is_active,
            created_at,
            updated_at,
        }
    }

    pub fn with_id(mut self, id: IndependentKeyId) -> Self {
        self.id = Some(id);
        self
    }

    // Getters
    pub fn id(&self) -> Option<&IndependentKeyId> {
        self.id.as_ref()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn provider_type(&self) -> &KeyProviderType {
        &self.provider_type
    }

    pub fn custom_provider_name(&self) -> Option<&str> {
        self.custom_provider_name.as_deref()
    }

    pub fn provider_display_name(&self) -> &str {
        if let Some(custom_name) = &self.custom_provider_name {
            custom_name
        } else {
            self.provider_type.display_name()
        }
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn organization_id(&self) -> Option<&str> {
        self.organization_id.as_deref()
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn masked_key(&self) -> String {
        if self.api_key.len() <= 12 {
            return "*".repeat(self.api_key.len());
        }
        format!("{}...{}", &self.api_key[..8], &self.api_key[self.api_key.len() - 4..])
    }

    // Business logic
    pub fn update(
        &mut self,
        name: Option<String>,
        api_key: Option<String>,
        base_url: Option<String>,
        organization_id: Option<String>,
        description: Option<String>,
    ) {
        if let Some(n) = name {
            self.name = n;
        }
        if let Some(k) = api_key {
            self.api_key = k;
        }
        if let Some(u) = base_url {
            self.base_url = u;
        }
        if organization_id.is_some() {
            self.organization_id = organization_id;
        }
        if description.is_some() {
            self.description = description;
        }
        self.updated_at = Utc::now();
    }

    pub fn set_active(&mut self, active: bool) {
        self.is_active = active;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_openai_key() {
        let key = IndependentApiKey::create(IndependentApiKeyConfig {
            name: "My OpenAI Key".to_string(),
            provider_type: KeyProviderType::OpenAI,
            custom_provider_name: None,
            api_key: "sk-test123456".to_string(),
            base_url: None,
            organization_id: None,
            description: Some("Test key".to_string()),
        });

        assert_eq!(key.name(), "My OpenAI Key");
        assert_eq!(key.base_url(), "https://api.openai.com/v1");
        assert_eq!(key.masked_key(), "sk-test1...3456");
        assert!(key.is_active());
    }

    #[test]
    fn test_create_custom_key() {
        let key = IndependentApiKey::create(IndependentApiKeyConfig {
            name: "Custom API".to_string(),
            provider_type: KeyProviderType::Custom,
            custom_provider_name: Some("MyProvider".to_string()),
            api_key: "custom-key-123".to_string(),
            base_url: Some("https://custom.api.com/v1".to_string()),
            organization_id: None,
            description: None,
        });

        assert_eq!(key.provider_display_name(), "MyProvider");
        assert_eq!(key.base_url(), "https://custom.api.com/v1");
    }

    #[test]
    fn test_update_key() {
        let mut key = IndependentApiKey::create(IndependentApiKeyConfig {
            name: "Test".to_string(),
            provider_type: KeyProviderType::OpenAI,
            custom_provider_name: None,
            api_key: "sk-old".to_string(),
            base_url: None,
            organization_id: None,
            description: None,
        });

        key.update(
            Some("Updated Name".to_string()),
            Some("sk-new".to_string()),
            None,
            None,
            Some(Some("New description".to_string())),
        );

        assert_eq!(key.name(), "Updated Name");
        assert_eq!(key.api_key(), "sk-new");
        assert_eq!(key.description(), Some("New description"));
    }
}
