use tauri::State;

use crate::application::dtos::{ProviderNodeDto, TokenDto};
use crate::presentation::state::AppState;
use neuradock_domain::shared::AccountId;
use neuradock_domain::check_in::Provider;

#[tauri::command]
#[specta::specta]
pub async fn fetch_account_tokens(
    account_id: String,
    force_refresh: bool,
    state: State<'_, AppState>,
) -> Result<Vec<TokenDto>, String> {
    log::info!("fetch_account_tokens called: account_id={}, force_refresh={}", account_id, force_refresh);
    let account_id = AccountId::from_string(&account_id);

    // Fetch tokens from service
    let tokens = state
        .token_service
        .fetch_and_cache_tokens(&account_id, force_refresh)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch tokens: {}", e);
            e.to_string()
        })?;

    log::info!("Fetched {} tokens for account {}", tokens.len(), account_id);

    // Get account info to fill DTO
    let account = state
        .account_repo
        .find_by_id(&account_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Account not found".to_string())?;

    // Get provider info
    let provider_id = account.provider_id();
    let providers = crate::presentation::commands::get_builtin_providers();
    let provider = providers
        .get(&provider_id.to_string())
        .ok_or_else(|| format!("Provider {} not found", provider_id))?;

    // Convert to DTOs
    let dtos = tokens
        .iter()
        .map(|token| {
            TokenDto::from_domain(
                token,
                account.name().to_string(),
                provider.name().to_string(),
            )
        })
        .collect();

    Ok(dtos)
}

#[tauri::command]
#[specta::specta]
pub async fn configure_claude_global(
    token_id: i64,
    account_id: String,
    base_url: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let account_id = AccountId::from_string(&account_id);
    let token_id = neuradock_domain::token::TokenId::new(token_id);

    // Get token from cache
    let tokens = state
        .token_service
        .get_cached_tokens(&account_id)
        .await
        .map_err(|e| e.to_string())?;

    let token = tokens
        .iter()
        .find(|t| t.id() == &token_id)
        .ok_or_else(|| "Token not found".to_string())?;

    // Configure to Claude Code
    let result = state
        .claude_config_service
        .configure_global(token, &base_url)
        .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn generate_claude_temp_commands(
    token_id: i64,
    account_id: String,
    base_url: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let account_id = AccountId::from_string(&account_id);
    let token_id = neuradock_domain::token::TokenId::new(token_id);

    // Get token from cache
    let tokens = state
        .token_service
        .get_cached_tokens(&account_id)
        .await
        .map_err(|e| e.to_string())?;

    let token = tokens
        .iter()
        .find(|t| t.id() == &token_id)
        .ok_or_else(|| "Token not found".to_string())?;

    // Generate temp commands
    let commands = state
        .claude_config_service
        .generate_temp_commands(token, &base_url)
        .map_err(|e| e.to_string())?;

    Ok(commands)
}

#[tauri::command]
#[specta::specta]
pub async fn get_provider_nodes(provider_id: String, state: State<'_, AppState>) -> Result<Vec<ProviderNodeDto>, String> {
    let mut nodes = vec![];

    // Add built-in nodes
    match provider_id.as_str() {
        "anyrouter" => {
            nodes.extend(vec![
                ProviderNodeDto {
                    id: "anytop".to_string(),
                    name: "AnyRouter Top".to_string(),
                    base_url: "https://anyrouter.top".to_string(),
                },
                ProviderNodeDto {
                    id: "rainapp".to_string(),
                    name: "RainApp".to_string(),
                    base_url: "https://pmpjfbhq.cn-nb1.rainapp.top".to_string(),
                },
                ProviderNodeDto {
                    id: "cspok".to_string(),
                    name: "CSPok".to_string(),
                    base_url: "https://c.cspok.cn".to_string(),
                },
                ProviderNodeDto {
                    id: "shanghai".to_string(),
                    name: "Shanghai".to_string(),
                    base_url: "https://a-ocnfniawgw.cn-shanghai.fcapp.run".to_string(),
                },
            ]);
        }
        "agentrouter" => {
            nodes.push(ProviderNodeDto {
                id: "agentrouter".to_string(),
                name: "AgentRouter".to_string(),
                base_url: "https://agentrouter.org".to_string(),
            });
        }
        _ => return Err("Unknown provider".to_string()),
    }

    // Add custom nodes
    let provider_id_obj = neuradock_domain::shared::ProviderId::from_string(&provider_id);
    let custom_nodes = state
        .custom_node_repo
        .find_by_provider(&provider_id_obj)
        .await
        .map_err(|e| e.to_string())?;

    for custom_node in custom_nodes {
        nodes.push(ProviderNodeDto {
            id: format!("custom_{}", custom_node.id().value()),
            name: custom_node.name().to_string(),
            base_url: custom_node.base_url().to_string(),
        });
    }

    Ok(nodes)
}

#[tauri::command]
#[specta::specta]
pub async fn add_custom_node(
    provider_id: String,
    name: String,
    base_url: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let provider_id_obj = neuradock_domain::shared::ProviderId::from_string(&provider_id);
    let node = neuradock_domain::custom_node::CustomProviderNode::create(
        provider_id_obj,
        name.clone(),
        base_url.clone(),
    );

    state
        .custom_node_repo
        .create(&node)
        .await
        .map_err(|e| e.to_string())?;

    Ok(format!("Custom node '{}' added successfully", name))
}

#[tauri::command]
#[specta::specta]
pub async fn delete_custom_node(
    node_id: i64,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let id = neuradock_domain::custom_node::CustomNodeId::new(node_id);

    state
        .custom_node_repo
        .delete(&id)
        .await
        .map_err(|e| e.to_string())?;

    Ok("Custom node deleted successfully".to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn configure_codex_global(
    token_id: i64,
    account_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let account_id = AccountId::from_string(&account_id);
    let token_id = neuradock_domain::token::TokenId::new(token_id);

    // Get token from cache
    let tokens = state
        .token_service
        .get_cached_tokens(&account_id)
        .await
        .map_err(|e| e.to_string())?;

    let token = tokens
        .iter()
        .find(|t| t.id() == &token_id)
        .ok_or_else(|| "Token not found".to_string())?;

    // Configure to Codex
    let result = state
        .codex_config_service
        .configure_global(token)
        .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn generate_codex_temp_commands(
    token_id: i64,
    account_id: String,
    base_url: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let account_id = AccountId::from_string(&account_id);
    let token_id = neuradock_domain::token::TokenId::new(token_id);

    // Get token from cache
    let tokens = state
        .token_service
        .get_cached_tokens(&account_id)
        .await
        .map_err(|e| e.to_string())?;

    let token = tokens
        .iter()
        .find(|t| t.id() == &token_id)
        .ok_or_else(|| "Token not found".to_string())?;

    // Generate temp commands
    let commands = state
        .codex_config_service
        .generate_temp_commands(token, &base_url)
        .map_err(|e| e.to_string())?;

    Ok(commands)
}

/// Check if models are compatible with a given AI tool
/// Returns (is_compatible, warning_message)
#[tauri::command]
#[specta::specta]
pub fn check_model_compatibility(
    models: Vec<String>,
    tool: String, // "claude" or "codex"
) -> Result<(bool, String), String> {
    let models_lower: Vec<String> = models.iter().map(|m| m.to_lowercase()).collect();

    match tool.as_str() {
        "claude" => {
            // Claude is compatible with claude models, glm, deepseek
            // Not compatible with gpt models
            let has_gpt = models_lower.iter().any(|m|
                m.contains("gpt") || m.contains("openai") || m.contains("o1")
            );

            let has_compatible = models_lower.iter().any(|m|
                m.contains("claude") || m.contains("glm") || m.contains("deepseek")
            );

            if has_gpt && !has_compatible {
                return Ok((false, "This token only supports GPT models which are not compatible with Claude Code. Please configure for Codex instead.".to_string()));
            } else if has_gpt {
                return Ok((true, "Warning: This token supports GPT models which may not work properly with Claude Code. Consider using Codex for GPT models.".to_string()));
            }

            Ok((true, String::new()))
        }
        "codex" => {
            // Codex is compatible with OpenAI/GPT models
            let has_openai = models_lower.iter().any(|m|
                m.contains("gpt") || m.contains("openai") || m.contains("o1")
            );

            if !has_openai {
                return Ok((true, "Note: This token may not contain OpenAI/GPT models. Codex works best with OpenAI-compatible APIs.".to_string()));
            }

            Ok((true, String::new()))
        }
        _ => Err("Unknown AI tool".to_string()),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn clear_claude_global(
    state: State<'_, AppState>,
) -> Result<String, String> {
    state
        .claude_config_service
        .clear_global()
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn clear_codex_global(
    state: State<'_, AppState>,
) -> Result<String, String> {
    state
        .codex_config_service
        .clear_global()
        .map_err(|e| e.to_string())
}
