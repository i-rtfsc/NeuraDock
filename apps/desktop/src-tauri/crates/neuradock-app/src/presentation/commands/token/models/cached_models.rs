use crate::presentation::error::CommandError;
use tauri::State;

use crate::presentation::state::Repositories;
#[tauri::command]
#[specta::specta]
pub async fn get_cached_provider_models(
    provider_id: String,
    repositories: State<'_, Repositories>,
) -> Result<Vec<String>, CommandError> {
    log::info!("get_cached_provider_models: provider_id={}", provider_id);

    let cached = repositories
        .provider_models
        .find_by_provider(&provider_id)
        .await
        .map_err(CommandError::from)?;

    Ok(cached.map(|c| c.models).unwrap_or_default())
}
