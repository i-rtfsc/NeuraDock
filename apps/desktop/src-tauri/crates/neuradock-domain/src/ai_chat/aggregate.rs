use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;
use uuid::Uuid;

use crate::shared::DomainError;

/// Unique identifier for an AI chat service
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
pub struct AiChatServiceId(String);

impl AiChatServiceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_string(s: &str) -> Self {
        Self(s.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AiChatServiceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for AiChatServiceId {
    fn default() -> Self {
        Self::new()
    }
}

/// AI Chat Service aggregate
/// Represents a chat service that can be embedded as a WebView
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AiChatService {
    id: AiChatServiceId,
    name: String,
    url: String,
    icon: Option<String>,
    is_builtin: bool,
    is_enabled: bool,
    sort_order: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl AiChatService {
    /// Create a new custom AI chat service
    pub fn new(name: String, url: String, icon: Option<String>) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::Validation(
                "Service name cannot be empty".to_string(),
            ));
        }

        if url.trim().is_empty() {
            return Err(DomainError::Validation(
                "Service URL cannot be empty".to_string(),
            ));
        }

        // Validate URL format
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(DomainError::Validation(
                "Service URL must start with http:// or https://".to_string(),
            ));
        }

        let now = Utc::now();
        Ok(Self {
            id: AiChatServiceId::new(),
            name: name.trim().to_string(),
            url: url.trim().to_string(),
            icon,
            is_builtin: false,
            is_enabled: true,
            sort_order: 0,
            created_at: now,
            updated_at: now,
        })
    }

    /// Create a built-in AI chat service (used for seeding)
    pub fn new_builtin(
        id: &str,
        name: String,
        url: String,
        icon: Option<String>,
        sort_order: i32,
        is_enabled: bool,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: AiChatServiceId::from_string(id),
            name,
            url,
            icon,
            is_builtin: true,
            is_enabled,
            sort_order,
            created_at: now,
            updated_at: now,
        }
    }

    /// Restore from persistence
    pub fn restore(
        id: AiChatServiceId,
        name: String,
        url: String,
        icon: Option<String>,
        is_builtin: bool,
        is_enabled: bool,
        sort_order: i32,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            url,
            icon,
            is_builtin,
            is_enabled,
            sort_order,
            created_at,
            updated_at,
        }
    }

    // Getters
    pub fn id(&self) -> &AiChatServiceId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn icon(&self) -> Option<&str> {
        self.icon.as_deref()
    }

    pub fn is_builtin(&self) -> bool {
        self.is_builtin
    }

    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    pub fn sort_order(&self) -> i32 {
        self.sort_order
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    // Mutations
    pub fn update(
        &mut self,
        name: String,
        url: String,
        icon: Option<String>,
    ) -> Result<(), DomainError> {
        if self.is_builtin {
            return Err(DomainError::Validation(
                "Cannot modify built-in service".to_string(),
            ));
        }

        if name.trim().is_empty() {
            return Err(DomainError::Validation(
                "Service name cannot be empty".to_string(),
            ));
        }

        if url.trim().is_empty() {
            return Err(DomainError::Validation(
                "Service URL cannot be empty".to_string(),
            ));
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(DomainError::Validation(
                "Service URL must start with http:// or https://".to_string(),
            ));
        }

        self.name = name.trim().to_string();
        self.url = url.trim().to_string();
        self.icon = icon;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn toggle(&mut self) {
        self.is_enabled = !self.is_enabled;
        self.updated_at = Utc::now();
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.is_enabled = enabled;
        self.updated_at = Utc::now();
    }

    pub fn set_sort_order(&mut self, order: i32) {
        self.sort_order = order;
        self.updated_at = Utc::now();
    }
}
