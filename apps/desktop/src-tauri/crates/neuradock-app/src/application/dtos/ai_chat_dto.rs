use serde::{Deserialize, Serialize};
use specta::Type;

use neuradock_domain::ai_chat::AiChatService;

/// DTO for AI Chat Service
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AiChatServiceDto {
    pub id: String,
    pub name: String,
    pub url: String,
    pub icon: Option<String>,
    pub is_builtin: bool,
    pub is_enabled: bool,
    pub sort_order: i32,
}

impl From<AiChatService> for AiChatServiceDto {
    fn from(service: AiChatService) -> Self {
        Self {
            id: service.id().as_str().to_string(),
            name: service.name().to_string(),
            url: service.url().to_string(),
            icon: service.icon().map(|s| s.to_string()),
            is_builtin: service.is_builtin(),
            is_enabled: service.is_enabled(),
            sort_order: service.sort_order(),
        }
    }
}

impl From<&AiChatService> for AiChatServiceDto {
    fn from(service: &AiChatService) -> Self {
        Self {
            id: service.id().as_str().to_string(),
            name: service.name().to_string(),
            url: service.url().to_string(),
            icon: service.icon().map(|s| s.to_string()),
            is_builtin: service.is_builtin(),
            is_enabled: service.is_enabled(),
            sort_order: service.sort_order(),
        }
    }
}

/// Input for creating a custom AI chat service
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CreateAiChatServiceInput {
    pub name: String,
    pub url: String,
    pub icon: Option<String>,
}

/// Input for updating an AI chat service
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UpdateAiChatServiceInput {
    pub id: String,
    pub name: String,
    pub url: String,
    pub icon: Option<String>,
}

/// Input for reordering AI chat services
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ReorderAiChatServicesInput {
    /// List of service IDs in the desired order
    pub service_ids: Vec<String>,
}
