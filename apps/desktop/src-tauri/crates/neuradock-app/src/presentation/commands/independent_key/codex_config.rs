use tauri::State;

use crate::application::services::token::CodexConfigService;
use crate::presentation::error::CommandError;
use crate::presentation::state::Repositories;
use neuradock_domain::independent_key::IndependentKeyId;

/// Configure independent API key to Codex globally
#[tauri::command]
#[specta::specta]
pub async fn configure_independent_key_codex(
    key_id: i64,
    model: Option<String>,
    context1m: Option<bool>,
    repositories: State<'_, Repositories>,
) -> Result<String, CommandError> {
    let id = IndependentKeyId::new(key_id);
    let key = repositories
        .independent_key
        .find_by_id(&id)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found(format!("Key with ID {} not found", key_id)))?;
    if !key.is_active() {
        return Err(CommandError::validation(
            "Cannot configure inactive API key. Please enable it first.",
        ));
    }
    CodexConfigService::new()
        .configure_global_with_key(
            key.api_key(),
            key.base_url(),
            model.as_deref(),
            context1m.unwrap_or(false),
        )
        .map_err(CommandError::from)
}

/// Configure independent API key to Codex using isolated files
#[tauri::command]
#[specta::specta]
pub async fn configure_independent_key_codex_isolated(
    key_id: i64,
    model: Option<String>,
    context1m: Option<bool>,
    repositories: State<'_, Repositories>,
) -> Result<String, CommandError> {
    let id = IndependentKeyId::new(key_id);
    let key = repositories
        .independent_key
        .find_by_id(&id)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found(format!("Key with ID {} not found", key_id)))?;
    if !key.is_active() {
        return Err(CommandError::validation(
            "Cannot configure inactive API key. Please enable it first.",
        ));
    }
    CodexConfigService::new()
        .configure_isolated_with_key(
            key.api_key(),
            key.base_url(),
            model.as_deref(),
            context1m.unwrap_or(false),
        )
        .map_err(CommandError::from)
}

/// Generate temporary Codex commands for independent API key
#[tauri::command]
#[specta::specta]
pub async fn generate_independent_key_codex_temp(
    key_id: i64,
    model: Option<String>,
    context1m: Option<bool>,
    repositories: State<'_, Repositories>,
) -> Result<String, CommandError> {
    let id = IndependentKeyId::new(key_id);
    let key = repositories
        .independent_key
        .find_by_id(&id)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found(format!("Key with ID {} not found", key_id)))?;
    CodexConfigService::new()
        .generate_temp_commands_with_key(
            key.api_key(),
            key.base_url(),
            model.as_deref(),
            context1m.unwrap_or(false),
        )
        .map_err(CommandError::from)
}
