use tauri::State;

use crate::application::dtos::{
    CreateIndependentKeyInput, IndependentKeyDto, UpdateIndependentKeyInput,
};
use crate::presentation::error::CommandError;
use crate::presentation::state::AppState;
use neuradock_domain::independent_key::{
    IndependentApiKey, IndependentApiKeyConfig, IndependentKeyId, KeyProviderType,
};

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

    keys.iter()
        .map(IndependentKeyDto::try_from_domain)
        .collect::<Result<Vec<_>, _>>()
        .map_err(CommandError::from)
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

    key.as_ref()
        .map(IndependentKeyDto::try_from_domain)
        .transpose()
        .map_err(CommandError::from)
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
    let key = IndependentApiKey::create(IndependentApiKeyConfig {
        name: input.name,
        provider_type,
        custom_provider_name: input.custom_provider_name,
        api_key: input.api_key,
        base_url: input.base_url,
        organization_id: input.organization_id,
        description: input.description,
    });

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

    IndependentKeyDto::try_from_domain(&created_key).map_err(CommandError::from)
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

    IndependentKeyDto::try_from_domain(&key).map_err(CommandError::from)
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

    IndependentKeyDto::try_from_domain(&key).map_err(CommandError::from)
}
