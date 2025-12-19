use tauri::State;

use crate::application::dtos::{
    CreateIndependentKeyInput, IndependentKeyDto, UpdateIndependentKeyInput,
};
use crate::presentation::error::CommandError;
use crate::presentation::state::AppState;
use neuradock_domain::independent_key::{IndependentApiKey, IndependentKeyId, KeyProviderType};

#[tauri::command]
#[specta::specta]
pub async fn get_all_independent_keys(
    state: State<'_, AppState>,
) -> Result<Vec<IndependentKeyDto>, CommandError> {
    let keys = state
        .independent_key_repo
        .find_all()
        .await
        .map_err(CommandError::from)?;

    Ok(keys.iter().map(IndependentKeyDto::from_domain).collect())
}

#[tauri::command]
#[specta::specta]
pub async fn get_independent_key_by_id(
    key_id: i64,
    state: State<'_, AppState>,
) -> Result<Option<IndependentKeyDto>, CommandError> {
    let id = IndependentKeyId::new(key_id);
    let key = state
        .independent_key_repo
        .find_by_id(&id)
        .await
        .map_err(CommandError::from)?;

    Ok(key.as_ref().map(IndependentKeyDto::from_domain))
}

#[tauri::command]
#[specta::specta]
pub async fn create_independent_key(
    input: CreateIndependentKeyInput,
    state: State<'_, AppState>,
) -> Result<IndependentKeyDto, CommandError> {
    // Validate provider type
    let provider_type = KeyProviderType::from_str(&input.provider_type)
        .ok_or_else(|| CommandError::validation(format!("Invalid provider_type: {}", input.provider_type)))?;

    // Validate custom provider name for custom type
    if provider_type == KeyProviderType::Custom && input.custom_provider_name.is_none() {
        return Err(CommandError::validation("custom_provider_name is required when provider_type is 'custom'"));
    }

    // Create domain object
    let key = IndependentApiKey::create(
        input.name,
        provider_type,
        input.custom_provider_name,
        input.api_key,
        input.base_url,
        input.organization_id,
        input.description,
    );

    // Save to database
    let id = state
        .independent_key_repo
        .create(&key)
        .await
        .map_err(CommandError::from)?;

    // Retrieve the created key
    let created_key = state
        .independent_key_repo
        .find_by_id(&id)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::infrastructure("Failed to retrieve created key"))?;

    Ok(IndependentKeyDto::from_domain(&created_key))
}

#[tauri::command]
#[specta::specta]
pub async fn update_independent_key(
    input: UpdateIndependentKeyInput,
    state: State<'_, AppState>,
) -> Result<IndependentKeyDto, CommandError> {
    let id = IndependentKeyId::new(input.key_id);

    // Retrieve existing key
    let mut key = state
        .independent_key_repo
        .find_by_id(&id)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found(format!("Key with ID {} not found", input.key_id)))?;

    // Update fields
    key.update(
        input.name,
        input.api_key,
        input.base_url,
        input.organization_id,
        input.description,
    );

    // Save changes
    state
        .independent_key_repo
        .update(&key)
        .await
        .map_err(CommandError::from)?;

    Ok(IndependentKeyDto::from_domain(&key))
}

#[tauri::command]
#[specta::specta]
pub async fn delete_independent_key(
    key_id: i64,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    let id = IndependentKeyId::new(key_id);

    state
        .independent_key_repo
        .delete(&id)
        .await
        .map_err(CommandError::from)?;

    Ok(format!("Independent key {} deleted successfully", key_id))
}

#[tauri::command]
#[specta::specta]
pub async fn toggle_independent_key(
    key_id: i64,
    is_active: bool,
    state: State<'_, AppState>,
) -> Result<IndependentKeyDto, CommandError> {
    let id = IndependentKeyId::new(key_id);

    let mut key = state
        .independent_key_repo
        .find_by_id(&id)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found(format!("Key with ID {} not found", key_id)))?;

    key.set_active(is_active);

    state
        .independent_key_repo
        .update(&key)
        .await
        .map_err(CommandError::from)?;

    Ok(IndependentKeyDto::from_domain(&key))
}

/// Configure independent API key to Claude Code globally
#[tauri::command]
#[specta::specta]
pub async fn configure_independent_key_claude(
    key_id: i64,
    model: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    let id = IndependentKeyId::new(key_id);

    // Get the independent key
    let key = state
        .independent_key_repo
        .find_by_id(&id)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found(format!("Key with ID {} not found", key_id)))?;

    // Check if key is active
    if !key.is_active() {
        return Err(CommandError::validation("Cannot configure inactive API key. Please enable it first."));
    }

    // Call Claude config service directly with API key
    use crate::application::services::token::ClaudeConfigService;
    let service = ClaudeConfigService::new();
    service
        .configure_global_with_key(key.api_key(), key.base_url(), model.as_deref())
        .map_err(CommandError::from)
}

/// Generate temporary Claude Code commands for independent API key
#[tauri::command]
#[specta::specta]
pub async fn generate_independent_key_claude_temp(
    key_id: i64,
    model: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    let id = IndependentKeyId::new(key_id);

    // Get the independent key
    let key = state
        .independent_key_repo
        .find_by_id(&id)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found(format!("Key with ID {} not found", key_id)))?;

    // Generate temp commands
    use crate::application::services::token::ClaudeConfigService;
    let service = ClaudeConfigService::new();
    service
        .generate_temp_commands_with_key(key.api_key(), key.base_url(), model.as_deref())
        .map_err(CommandError::from)
}

/// Configure independent API key to Codex globally
#[tauri::command]
#[specta::specta]
pub async fn configure_independent_key_codex(
    key_id: i64,
    model: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    let id = IndependentKeyId::new(key_id);

    // Get the independent key
    let key = state
        .independent_key_repo
        .find_by_id(&id)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found(format!("Key with ID {} not found", key_id)))?;

    // Check if key is active
    if !key.is_active() {
        return Err(CommandError::validation("Cannot configure inactive API key. Please enable it first."));
    }

    // Call Codex config service with API key
    use crate::application::services::token::CodexConfigService;
    let service = CodexConfigService::new();
    service
        .configure_global_with_key(key.api_key(), key.base_url(), model.as_deref())
        .map_err(CommandError::from)
}

/// Generate temporary Codex commands for independent API key
#[tauri::command]
#[specta::specta]
pub async fn generate_independent_key_codex_temp(
    key_id: i64,
    model: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    let id = IndependentKeyId::new(key_id);

    // Get the independent key
    let key = state
        .independent_key_repo
        .find_by_id(&id)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found(format!("Key with ID {} not found", key_id)))?;

    // Generate temp commands
    use crate::application::services::token::CodexConfigService;
    let service = CodexConfigService::new();
    service
        .generate_temp_commands_with_key(key.api_key(), key.base_url(), model.as_deref())
        .map_err(CommandError::from)
}
