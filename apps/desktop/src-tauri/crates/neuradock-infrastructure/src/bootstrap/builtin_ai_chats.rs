use std::sync::Arc;

use neuradock_domain::ai_chat::{AiChatService, AiChatServiceId, AiChatServiceRepository};
use neuradock_domain::shared::DomainError;
use serde::Deserialize;
use tracing::info;

/// Built-in AI chat service definition from JSON config
#[derive(Debug, Deserialize)]
struct BuiltinAiChatConfig {
    id: String,
    name: String,
    url: String,
    icon: String,
    sort_order: i32,
    is_enabled: bool,
}

/// Load built-in AI chat services from embedded JSON config file
fn load_builtin_ai_chats() -> Vec<BuiltinAiChatConfig> {
    const CONFIG_JSON: &str = include_str!("../../../../config/ai_chat/builtin_services.json");
    
    serde_json::from_str(CONFIG_JSON).unwrap_or_else(|e| {
        tracing::error!("Failed to parse builtin_services.json: {}", e);
        Vec::new()
    })
}

/// Seed built-in AI chat services if they don't already exist.
/// Only inserts services that are missing; never overwrites user changes.
pub async fn seed_builtin_ai_chats(
    repo: Arc<dyn AiChatServiceRepository>,
) -> Result<(), DomainError> {
    let builtin_services = load_builtin_ai_chats();
    let mut seeded_count = 0;

    for builtin in &builtin_services {
        let id = AiChatServiceId::from_string(&builtin.id);
        if !repo.exists(&id).await? {
            let service = AiChatService::new_builtin(
                &builtin.id,
                builtin.name.clone(),
                builtin.url.clone(),
                Some(builtin.icon.clone()),
                builtin.sort_order,
                builtin.is_enabled,
            );
            repo.save(&service).await?;
            seeded_count += 1;
            info!("Seeded built-in AI chat service: {}", builtin.name);
        }
    }

    if seeded_count > 0 {
        info!("Seeded {} built-in AI chat service(s)", seeded_count);
    }

    Ok(())
}
