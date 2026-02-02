use std::sync::Arc;

use neuradock_domain::ai_chat::{AiChatService, AiChatServiceId, AiChatServiceRepository};
use neuradock_domain::shared::DomainError;
use tracing::info;

/// Built-in AI chat service definitions
struct BuiltinAiChat {
    id: &'static str,
    name: &'static str,
    url: &'static str,
    icon: &'static str,
    sort_order: i32,
    is_enabled: bool,
}

const BUILTIN_AI_CHATS: &[BuiltinAiChat] = &[
    BuiltinAiChat {
        id: "deepseek",
        name: "DeepSeek",
        url: "https://chat.deepseek.com/",
        icon: "deepseek",
        sort_order: 0,
        is_enabled: true,
    },
    BuiltinAiChat {
        id: "chatgpt",
        name: "ChatGPT",
        url: "https://chatgpt.com/",
        icon: "chatgpt",
        sort_order: 1,
        is_enabled: false,
    },
    BuiltinAiChat {
        id: "claude",
        name: "Claude",
        url: "https://claude.ai/",
        icon: "claude",
        sort_order: 2,
        is_enabled: false,
    },
    BuiltinAiChat {
        id: "gemini",
        name: "Gemini",
        url: "https://gemini.google.com/",
        icon: "gemini",
        sort_order: 3,
        is_enabled: false,
    },
];

/// Seed built-in AI chat services if they don't already exist.
/// Only inserts services that are missing; never overwrites user changes.
pub async fn seed_builtin_ai_chats(
    repo: Arc<dyn AiChatServiceRepository>,
) -> Result<(), DomainError> {
    let mut seeded_count = 0;

    for builtin in BUILTIN_AI_CHATS {
        let id = AiChatServiceId::from_string(builtin.id);
        if !repo.exists(&id).await? {
            let service = AiChatService::new_builtin(
                builtin.id,
                builtin.name.to_string(),
                builtin.url.to_string(),
                Some(builtin.icon.to_string()),
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
