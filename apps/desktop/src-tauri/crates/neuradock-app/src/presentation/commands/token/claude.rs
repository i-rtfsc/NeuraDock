use crate::presentation::error::CommandError;
use crate::presentation::state::AppState;
use neuradock_domain::shared::AccountId;
use tauri::State;

#[tauri::command]
#[specta::specta]
pub async fn configure_claude_global(
    token_id: i64,
    account_id: String,
    base_url: String,
    model: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    let account_id = AccountId::from_string(&account_id);
    let token_id = neuradock_domain::token::TokenId::new(token_id);

    // Get token from cache
    let tokens = state
        .token_service
        .get_cached_tokens(&account_id)
        .await
        .map_err(CommandError::from)?;

    let token = tokens
        .iter()
        .find(|t| t.id() == &token_id)
        .ok_or_else(|| CommandError::not_found("Token not found"))?;

    // Configure to Claude Code
    let result = state
        .claude_config_service
        .configure_global(token, &base_url, model.as_deref())
        .map_err(CommandError::from)?;

    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn generate_claude_temp_commands(
    token_id: i64,
    account_id: String,
    base_url: String,
    model: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    let account_id = AccountId::from_string(&account_id);
    let token_id = neuradock_domain::token::TokenId::new(token_id);

    // Get token from cache
    let tokens = state
        .token_service
        .get_cached_tokens(&account_id)
        .await
        .map_err(CommandError::from)?;

    let token = tokens
        .iter()
        .find(|t| t.id() == &token_id)
        .ok_or_else(|| CommandError::not_found("Token not found"))?;

    // Generate temp commands
    let commands = state
        .claude_config_service
        .generate_temp_commands(token, &base_url, model.as_deref())
        .map_err(CommandError::from)?;

    Ok(commands)
}

#[tauri::command]
#[specta::specta]
pub async fn clear_claude_global(state: State<'_, AppState>) -> Result<String, CommandError> {
    state
        .claude_config_service
        .clear_global()
        .map_err(CommandError::from)
}

/// Check if models are compatible with Claude Code
/// Returns (is_compatible, warning_message)
#[tauri::command]
#[specta::specta]
pub fn check_model_compatibility(
    models: Vec<String>,
    tool: String, // "claude" or "codex"
) -> Result<(bool, String), CommandError> {
    let models_lower: Vec<String> = models.iter().map(|m| m.to_lowercase()).collect();

    match tool.as_str() {
        "claude" => {
            // Claude is compatible with claude models, glm, deepseek
            // Not compatible with gpt models
            let has_gpt = models_lower
                .iter()
                .any(|m| m.contains("gpt") || m.contains("openai") || m.contains("o1"));

            let has_compatible = models_lower
                .iter()
                .any(|m| m.contains("claude") || m.contains("glm") || m.contains("deepseek"));

            if has_gpt && !has_compatible {
                return Ok((false, "This token only supports GPT models which are not compatible with Claude Code. Please configure for Codex instead.".to_string()));
            } else if has_gpt {
                return Ok((true, "Warning: This token supports GPT models which may not work properly with Claude Code. Consider using Codex for GPT models.".to_string()));
            }

            Ok((true, String::new()))
        }
        "codex" => {
            // Codex is compatible with OpenAI/GPT models
            let has_openai = models_lower
                .iter()
                .any(|m| m.contains("gpt") || m.contains("openai") || m.contains("o1"));

            if !has_openai {
                return Ok((true, "Note: This token may not contain OpenAI/GPT models. Codex works best with OpenAI-compatible APIs.".to_string()));
            }

            Ok((true, String::new()))
        }
        _ => Err(CommandError::validation("Unknown AI tool")),
    }
}
