use crate::presentation::error::CommandError;
use neuradock_infrastructure::logging::{log_from_frontend as log_fe, FrontendLog};

use tauri::Manager;
use tauri_plugin_opener::OpenerExt;

/// Get application version information
#[tauri::command]
#[specta::specta]
pub fn get_app_version() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let profile = if cfg!(debug_assertions) {
        "Debug"
    } else {
        "Release"
    };
    format!("{} ({})", version, profile)
}

/// Log from frontend
#[tauri::command]
#[specta::specta]
pub fn log_from_frontend(level: String, target: String, message: String, fields: Option<String>) {
    // Parse fields JSON string
    let parsed_fields = fields.and_then(|f| serde_json::from_str(&f).ok());

    let log = FrontendLog {
        level,
        target,
        message,
        fields: parsed_fields,
    };

    log_fe(log);
}

/// Open log directory in file explorer
#[tauri::command]
#[specta::specta]
pub async fn open_log_dir(app: tauri::AppHandle) -> Result<String, CommandError> {
    use neuradock_infrastructure::logging;

    let log_dir = logging::get_log_dir()
        .or_else(|| {
            // If not initialized yet, try to get default path
            app.path().app_log_dir().ok().map(|dir| dir.join("logs"))
        })
        .ok_or_else(|| CommandError::infrastructure("Failed to get log directory"))?;

    // Ensure directory exists
    std::fs::create_dir_all(&log_dir).map_err(|e| CommandError::from(e.to_string()))?;

    app.opener()
        .reveal_item_in_dir(&log_dir)
        .map_err(|e| CommandError::from(e.to_string()))?;

    Ok(log_dir.display().to_string())
}
