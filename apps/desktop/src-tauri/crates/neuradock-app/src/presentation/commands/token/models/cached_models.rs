use tauri::State;


use crate::presentation::state::AppState;
#[tauri::command]
#[specta::specta]
pub async fn get_cached_provider_models(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    log::info!("get_cached_provider_models: provider_id={}", provider_id);

    match state
        .provider_models_repo
        .find_by_provider(&provider_id)
        .await
    {
        Ok(Some(cached)) => {
            log::info!(
                "Found {} cached models for provider {}",
                cached.models.len(),
                provider_id
            );
            Ok(cached.models)
        }
        Ok(None) => {
            log::info!("No cached models for provider {}", provider_id);
            Ok(vec![])
        }
        Err(e) => {
            log::error!("Failed to get cached models: {}", e);
            Err(e.to_string())
        }
    }
}
