use crate::application::services::LogLevel;
use crate::presentation::state::AppState;
use tauri::State;

/// Get current log level
#[tauri::command]
#[specta::specta]
pub async fn get_log_level(state: State<'_, AppState>) -> Result<String, String> {
    let level = state.config_service.get_log_level();
    Ok(level.as_str().to_string())
}

/// Set log level
#[tauri::command]
#[specta::specta]
pub async fn set_log_level(level: String, state: State<'_, AppState>) -> Result<(), String> {
    let log_level = match level.to_lowercase().as_str() {
        "error" => LogLevel::Error,
        "warn" => LogLevel::Warn,
        "info" => LogLevel::Info,
        "debug" => LogLevel::Debug,
        "trace" => LogLevel::Trace,
        _ => {
            return Err(
                "Invalid log level. Must be one of: error, warn, info, debug, trace".to_string(),
            )
        }
    };

    state
        .config_service
        .set_log_level(log_level)
        .map_err(|e| format!("Failed to save log level: {}", e))?;
    Ok(())
}
