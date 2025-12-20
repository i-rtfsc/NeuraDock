use serde::{Deserialize, Serialize};
use specta::Type;
use neuradock_domain::shared::DomainError;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct IndependentKeyDto {
    pub id: i64,
    pub name: String,
    pub provider_type: String,
    pub provider_type_display: String,
    pub custom_provider_name: Option<String>,
    pub masked_key: String,
    pub base_url: String,
    pub organization_id: Option<String>,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl IndependentKeyDto {
    pub fn try_from_domain(
        key: &neuradock_domain::independent_key::IndependentApiKey,
    ) -> Result<Self, DomainError> {
        let id = key
            .id()
            .ok_or_else(|| DomainError::Infrastructure("Independent key is missing ID".to_string()))?
            .value();

        Ok(Self {
            id,
            name: key.name().to_string(),
            provider_type: key.provider_type().as_str().to_string(),
            provider_type_display: key.provider_type().display_name().to_string(),
            custom_provider_name: key.custom_provider_name().map(|s| s.to_string()),
            masked_key: key.masked_key(),
            base_url: key.base_url().to_string(),
            organization_id: key.organization_id().map(|s| s.to_string()),
            description: key.description().map(|s| s.to_string()),
            is_active: key.is_active(),
            created_at: key.created_at().to_rfc3339(),
            updated_at: key.updated_at().to_rfc3339(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CreateIndependentKeyInput {
    pub name: String,
    pub provider_type: String, // "openai", "anthropic", "custom"
    pub custom_provider_name: Option<String>, // Required if provider_type is "custom"
    pub api_key: String,
    pub base_url: Option<String>, // Optional, will use default if not provided
    pub organization_id: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UpdateIndependentKeyInput {
    pub key_id: i64,
    pub name: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub organization_id: Option<String>,
    pub description: Option<String>,
}
