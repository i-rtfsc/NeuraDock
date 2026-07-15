use crate::application::dtos::LegacyImportResultDto;
use crate::application::services::{LegacyImportError, LegacyMigrationService};
use crate::presentation::error::CommandError;
use tauri::Manager;

#[tauri::command]
#[specta::specta]
pub async fn detect_legacy_installation(app: tauri::AppHandle) -> Result<bool, CommandError> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| {
        CommandError::infrastructure(format!("Failed to get app data directory: {}", e))
    })?;

    Ok(LegacyMigrationService::detect_legacy_installation(
        &app_data_dir,
    ))
}

#[tauri::command]
#[specta::specta]
pub async fn import_legacy_data(
    app: tauri::AppHandle,
    overwrite: bool,
) -> Result<LegacyImportResultDto, CommandError> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| {
        CommandError::infrastructure(format!("Failed to get app data directory: {}", e))
    })?;

    LegacyMigrationService::import_legacy_data(&app_data_dir, overwrite).map_err(|e| match e {
        LegacyImportError::OverwriteRequired { existing } => CommandError::validation(format!(
            "LEGACY_IMPORT_OVERWRITE_REQUIRED:{}",
            existing.join(",")
        )),
        LegacyImportError::LegacyNotFound => {
            CommandError::validation("LEGACY_IMPORT_NOT_FOUND".to_string())
        }
        _ => CommandError::infrastructure(e.to_string()),
    })
}
