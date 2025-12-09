use serde::{Deserialize, Serialize};
use specta::Type;
use std::fmt;

use crate::shared::DomainError;

/// Notification channel ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
pub struct NotificationChannelId(String);

impl NotificationChannelId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(id: &str) -> Self {
        Self(id.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for NotificationChannelId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for NotificationChannelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Channel type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    /// Feishu (Lark) webhook
    Feishu,
    /// DingTalk webhook
    DingTalk,
    /// Email notification
    Email,
}

impl ChannelType {
    pub fn as_str(&self) -> &str {
        match self {
            ChannelType::Feishu => "feishu",
            ChannelType::DingTalk => "dingtalk",
            ChannelType::Email => "email",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, DomainError> {
        match s {
            "feishu" => Ok(ChannelType::Feishu),
            "dingtalk" => Ok(ChannelType::DingTalk),
            "email" => Ok(ChannelType::Email),
            _ => Err(DomainError::InvalidInput(format!(
                "Unknown channel type: {}",
                s
            ))),
        }
    }
}

impl fmt::Display for ChannelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChannelConfig {
    /// Feishu webhook configuration
    Feishu { webhook_key: String },
    /// DingTalk webhook configuration
    DingTalk {
        webhook_key: String,
        secret: Option<String>,
    },
    /// Email configuration
    Email {
        smtp_host: String,
        smtp_port: u16,
        username: String,
        password: String,
        from: String,
        to: Vec<String>,
    },
}

impl ChannelConfig {
    /// Validate configuration
    pub fn validate(&self) -> Result<(), DomainError> {
        match self {
            ChannelConfig::Feishu { webhook_key } => {
                if webhook_key.trim().is_empty() {
                    return Err(DomainError::InvalidInput(
                        "Feishu webhook_key cannot be empty".to_string(),
                    ));
                }
            }
            ChannelConfig::DingTalk { webhook_key, .. } => {
                if webhook_key.trim().is_empty() {
                    return Err(DomainError::InvalidInput(
                        "DingTalk webhook_key cannot be empty".to_string(),
                    ));
                }
            }
            ChannelConfig::Email {
                smtp_host,
                smtp_port,
                username,
                password,
                from,
                to,
            } => {
                if smtp_host.trim().is_empty() {
                    return Err(DomainError::InvalidInput(
                        "SMTP host cannot be empty".to_string(),
                    ));
                }
                if *smtp_port == 0 {
                    return Err(DomainError::InvalidInput(
                        "SMTP port must be greater than 0".to_string(),
                    ));
                }
                if username.trim().is_empty() {
                    return Err(DomainError::InvalidInput(
                        "Username cannot be empty".to_string(),
                    ));
                }
                if password.trim().is_empty() {
                    return Err(DomainError::InvalidInput(
                        "Password cannot be empty".to_string(),
                    ));
                }
                if from.trim().is_empty() {
                    return Err(DomainError::InvalidInput(
                        "From address cannot be empty".to_string(),
                    ));
                }
                if to.is_empty() {
                    return Err(DomainError::InvalidInput(
                        "To addresses cannot be empty".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Get channel type from config
    pub fn channel_type(&self) -> ChannelType {
        match self {
            ChannelConfig::Feishu { .. } => ChannelType::Feishu,
            ChannelConfig::DingTalk { .. } => ChannelType::DingTalk,
            ChannelConfig::Email { .. } => ChannelType::Email,
        }
    }

    /// Serialize to JSON string for database storage
    pub fn to_json(&self) -> Result<String, DomainError> {
        serde_json::to_string(self)
            .map_err(|e| DomainError::Serialization(format!("Failed to serialize config: {}", e)))
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, DomainError> {
        serde_json::from_str(json).map_err(|e| {
            DomainError::Deserialization(format!("Failed to deserialize config: {}", e))
        })
    }
}
