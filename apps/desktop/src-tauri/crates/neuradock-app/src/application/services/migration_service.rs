//! Legacy data migration from prior NeuraDock installations.
//!
//! Detects a previous installation's data directory (identified by the
//! `com.neuradock.app` bundle ID), stages database and encryption-salt
//! files into a temporary directory, then atomically applies them on the
//! next application startup before the database is opened.

use crate::application::dtos::{LegacyImportFilesDto, LegacyImportResultDto};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const LEGACY_IDENTIFIER: &str = "com.neuradock.app";

const PROD_DB_FILENAME: &str = "neuradock.db";
const DEV_DB_FILENAME: &str = "neuradock-dev.db";
const LEGACY_DEV_DB_FALLBACK_FILENAME: &str = "neuradock_dev.db";
const ENCRYPTION_SALT_FILENAME: &str = ".encryption_salt";

const STAGING_DIR_NAME: &str = ".legacy_import_staging";
const PENDING_MARKER_FILENAME: &str = ".legacy_import_pending.json";
const COMPLETED_MARKER_FILENAME: &str = ".legacy_import_completed.json";
const DISMISSED_MARKER_FILENAME: &str = ".legacy_import_dismissed.json";

/// Errors that can occur during the legacy import process.
#[derive(Debug, thiserror::Error)]
pub enum LegacyImportError {
    #[error("Invalid app data directory")]
    InvalidAppDataDir,

    #[error("Legacy installation data not found")]
    LegacyNotFound,

    #[error("Overwrite confirmation required")]
    OverwriteRequired { existing: Vec<String> },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to serialize marker: {0}")]
    SerializeMarker(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingLegacyImportMarker {
    version: u32,
    legacy_identifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompletedLegacyImportMarker {
    version: u32,
    legacy_identifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DismissedLegacyImportMarker {
    version: u32,
    legacy_identifier: String,
}

/// Stateless service that detects, stages, and applies data from a legacy
/// NeuraDock installation so users can upgrade without losing accounts.
pub struct LegacyMigrationService;

impl LegacyMigrationService {
    /// Returns `true` if a legacy data directory with importable files exists
    /// and no completed-import marker has been written yet.
    pub fn detect_legacy_installation(new_app_data_dir: &Path) -> bool {
        if Self::completed_marker_path(new_app_data_dir).exists() {
            return false;
        }

        // The dismissal marker lives beside the data it refers to, so wiping
        // the app data directory resets the prompt along with everything else.
        if Self::dismissed_marker_path(new_app_data_dir).exists() {
            return false;
        }

        let Ok(legacy_dir) = Self::legacy_dir_for(new_app_data_dir) else {
            return false;
        };

        if !legacy_dir.exists() {
            return false;
        }

        let has_prod_db = legacy_dir.join(PROD_DB_FILENAME).exists();
        let has_dev_db = legacy_dir.join(DEV_DB_FILENAME).exists()
            || legacy_dir.join(LEGACY_DEV_DB_FALLBACK_FILENAME).exists();
        let has_salt = legacy_dir.join(ENCRYPTION_SALT_FILENAME).exists();

        has_prod_db || has_dev_db || has_salt
    }

    #[cfg(test)]
    pub fn has_pending_import(new_app_data_dir: &Path) -> bool {
        Self::pending_marker_path(new_app_data_dir).exists()
    }

    /// Copies legacy files into a staging directory and writes a pending-import
    /// marker.  The staged files are applied on the next startup via
    /// [`apply_pending_import`](Self::apply_pending_import).
    ///
    /// # Errors
    /// Returns [`LegacyImportError::OverwriteRequired`] when target files
    /// already exist and `overwrite` is `false`.
    pub fn import_legacy_data(
        new_app_data_dir: &Path,
        overwrite: bool,
    ) -> Result<LegacyImportResultDto, LegacyImportError> {
        std::fs::create_dir_all(new_app_data_dir)?;

        if Self::pending_marker_path(new_app_data_dir).exists() {
            let legacy_dir = Self::legacy_dir_for(new_app_data_dir)?;
            return Ok(LegacyImportResultDto {
                legacy_dir: legacy_dir.display().to_string(),
                new_dir: new_app_data_dir.display().to_string(),
                staged: true,
                already_staged: true,
                files: LegacyImportFilesDto {
                    production_db: legacy_dir.join(PROD_DB_FILENAME).exists(),
                    development_db: legacy_dir.join(DEV_DB_FILENAME).exists()
                        || legacy_dir.join(LEGACY_DEV_DB_FALLBACK_FILENAME).exists(),
                    encryption_salt: legacy_dir.join(ENCRYPTION_SALT_FILENAME).exists(),
                },
                requires_restart: true,
            });
        }

        let legacy_dir = Self::legacy_dir_for(new_app_data_dir)?;
        if !legacy_dir.exists() {
            return Err(LegacyImportError::LegacyNotFound);
        }

        let legacy_prod_db = legacy_dir.join(PROD_DB_FILENAME);
        let legacy_dev_db = legacy_dir.join(DEV_DB_FILENAME);
        let legacy_dev_db_fallback = legacy_dir.join(LEGACY_DEV_DB_FALLBACK_FILENAME);
        let legacy_salt = legacy_dir.join(ENCRYPTION_SALT_FILENAME);

        let has_any_legacy_file = legacy_prod_db.exists()
            || legacy_dev_db.exists()
            || legacy_dev_db_fallback.exists()
            || legacy_salt.exists();
        if !has_any_legacy_file {
            return Err(LegacyImportError::LegacyNotFound);
        }

        let existing = Self::existing_target_files(new_app_data_dir);
        if !overwrite && !existing.is_empty() {
            return Err(LegacyImportError::OverwriteRequired { existing });
        }

        let staging_dir = new_app_data_dir.join(STAGING_DIR_NAME);
        if staging_dir.exists() {
            std::fs::remove_dir_all(&staging_dir)?;
        }
        std::fs::create_dir_all(&staging_dir)?;

        let mut staged_files = LegacyImportFilesDto {
            production_db: false,
            development_db: false,
            encryption_salt: false,
        };

        if legacy_prod_db.exists() {
            std::fs::copy(&legacy_prod_db, staging_dir.join(PROD_DB_FILENAME))?;
            staged_files.production_db = true;
        }
        if legacy_dev_db.exists() {
            std::fs::copy(&legacy_dev_db, staging_dir.join(DEV_DB_FILENAME))?;
            staged_files.development_db = true;
        } else if legacy_dev_db_fallback.exists() {
            std::fs::copy(&legacy_dev_db_fallback, staging_dir.join(DEV_DB_FILENAME))?;
            staged_files.development_db = true;
        }
        if legacy_salt.exists() {
            std::fs::copy(&legacy_salt, staging_dir.join(ENCRYPTION_SALT_FILENAME))?;
            staged_files.encryption_salt = true;
        }

        let marker = PendingLegacyImportMarker {
            version: 1,
            legacy_identifier: LEGACY_IDENTIFIER.to_string(),
        };
        let marker_json = serde_json::to_string_pretty(&marker)?;
        std::fs::write(Self::pending_marker_path(new_app_data_dir), marker_json)?;

        Ok(LegacyImportResultDto {
            legacy_dir: legacy_dir.display().to_string(),
            new_dir: new_app_data_dir.display().to_string(),
            staged: true,
            already_staged: false,
            files: staged_files,
            requires_restart: true,
        })
    }

    /// Applies a previously staged import, if one is pending.
    ///
    /// Must be called **before** opening the database or initialising
    /// encryption so the files can be safely overwritten.  Returns `true`
    /// when files were moved into place, `false` when there was nothing
    /// to apply.
    ///
    /// # Errors
    /// Returns an error on I/O failures during file moves.
    pub fn apply_pending_import(new_app_data_dir: &Path) -> Result<bool, LegacyImportError> {
        let marker_path = Self::pending_marker_path(new_app_data_dir);
        if !marker_path.exists() {
            return Ok(false);
        }

        let staging_dir = new_app_data_dir.join(STAGING_DIR_NAME);
        if !staging_dir.exists() {
            // Nothing to apply; clear the marker to prevent repeated attempts.
            let _ = std::fs::remove_file(&marker_path);
            return Ok(false);
        }

        Self::move_if_exists(
            &staging_dir.join(PROD_DB_FILENAME),
            &new_app_data_dir.join(PROD_DB_FILENAME),
        )?;
        Self::move_if_exists(
            &staging_dir.join(DEV_DB_FILENAME),
            &new_app_data_dir.join(DEV_DB_FILENAME),
        )?;
        Self::move_if_exists(
            &staging_dir.join(ENCRYPTION_SALT_FILENAME),
            &new_app_data_dir.join(ENCRYPTION_SALT_FILENAME),
        )?;

        let _ = std::fs::remove_file(&marker_path);
        let _ = std::fs::remove_dir_all(&staging_dir);

        let completed = CompletedLegacyImportMarker {
            version: 1,
            legacy_identifier: LEGACY_IDENTIFIER.to_string(),
        };
        let completed_json = serde_json::to_string_pretty(&completed)?;
        std::fs::write(
            Self::completed_marker_path(new_app_data_dir),
            completed_json,
        )?;

        Ok(true)
    }

    fn legacy_dir_for(new_app_data_dir: &Path) -> Result<PathBuf, LegacyImportError> {
        let parent = new_app_data_dir
            .parent()
            .ok_or(LegacyImportError::InvalidAppDataDir)?;
        Ok(parent.join(LEGACY_IDENTIFIER))
    }

    fn existing_target_files(new_app_data_dir: &Path) -> Vec<String> {
        let mut existing = Vec::new();
        for name in [PROD_DB_FILENAME, DEV_DB_FILENAME, ENCRYPTION_SALT_FILENAME] {
            if new_app_data_dir.join(name).exists() {
                existing.push(name.to_string());
            }
        }
        existing
    }

    fn pending_marker_path(new_app_data_dir: &Path) -> PathBuf {
        new_app_data_dir.join(PENDING_MARKER_FILENAME)
    }

    fn completed_marker_path(new_app_data_dir: &Path) -> PathBuf {
        new_app_data_dir.join(COMPLETED_MARKER_FILENAME)
    }

    fn dismissed_marker_path(new_app_data_dir: &Path) -> PathBuf {
        new_app_data_dir.join(DISMISSED_MARKER_FILENAME)
    }

    /// Records that the user dismissed the import prompt, suppressing future
    /// startup prompts until the app data directory is recreated. Manual
    /// import from Settings stays available regardless.
    pub fn dismiss_prompt(new_app_data_dir: &Path) -> Result<(), LegacyImportError> {
        std::fs::create_dir_all(new_app_data_dir)?;
        let marker = DismissedLegacyImportMarker {
            version: 1,
            legacy_identifier: LEGACY_IDENTIFIER.to_string(),
        };
        std::fs::write(
            Self::dismissed_marker_path(new_app_data_dir),
            serde_json::to_string_pretty(&marker)?,
        )?;
        Ok(())
    }

    fn move_if_exists(src: &Path, dest: &Path) -> Result<(), LegacyImportError> {
        if !src.exists() {
            return Ok(());
        }
        if dest.exists() {
            std::fs::remove_file(dest)?;
        }
        std::fs::rename(src, dest)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_file(path: &Path, content: &[u8]) {
        std::fs::create_dir_all(path.parent().expect("file has parent")).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn detect_legacy_installation_true_when_legacy_files_exist() {
        let tmp = TempDir::new().unwrap();
        let parent = tmp.path();

        let new_dir = parent.join("com.rtfsc.neuradock");
        let legacy_dir = parent.join(LEGACY_IDENTIFIER);

        write_file(&legacy_dir.join(PROD_DB_FILENAME), b"sqlite");

        assert!(LegacyMigrationService::detect_legacy_installation(&new_dir));
    }

    #[test]
    fn stage_and_apply_pending_import_moves_files_into_place() {
        let tmp = TempDir::new().unwrap();
        let parent = tmp.path();

        let new_dir = parent.join("com.rtfsc.neuradock");
        let legacy_dir = parent.join(LEGACY_IDENTIFIER);

        write_file(&legacy_dir.join(PROD_DB_FILENAME), b"prod-db");
        write_file(&legacy_dir.join(DEV_DB_FILENAME), b"dev-db");
        write_file(&legacy_dir.join(ENCRYPTION_SALT_FILENAME), b"salt");

        // Stage import
        let result = LegacyMigrationService::import_legacy_data(&new_dir, true).unwrap();
        assert!(result.staged);
        assert!(LegacyMigrationService::has_pending_import(&new_dir));

        // Apply on next startup
        assert!(LegacyMigrationService::apply_pending_import(&new_dir).unwrap());
        assert!(!LegacyMigrationService::has_pending_import(&new_dir));

        assert_eq!(
            std::fs::read(new_dir.join(PROD_DB_FILENAME)).unwrap(),
            b"prod-db"
        );
        assert_eq!(
            std::fs::read(new_dir.join(DEV_DB_FILENAME)).unwrap(),
            b"dev-db"
        );
        assert_eq!(
            std::fs::read(new_dir.join(ENCRYPTION_SALT_FILENAME)).unwrap(),
            b"salt"
        );

        // Completed marker should suppress future detection
        assert!(!LegacyMigrationService::detect_legacy_installation(
            &new_dir
        ));
    }

    #[test]
    fn dismiss_prompt_suppresses_detection_until_data_dir_reset() {
        let tmp = TempDir::new().unwrap();
        let parent = tmp.path();

        let new_dir = parent.join("com.rtfsc.neuradock");
        let legacy_dir = parent.join(LEGACY_IDENTIFIER);
        write_file(&legacy_dir.join(PROD_DB_FILENAME), b"sqlite");

        assert!(LegacyMigrationService::detect_legacy_installation(&new_dir));

        LegacyMigrationService::dismiss_prompt(&new_dir).unwrap();
        assert!(!LegacyMigrationService::detect_legacy_installation(
            &new_dir
        ));

        // Wiping the app data directory resets the prompt.
        std::fs::remove_dir_all(&new_dir).unwrap();
        assert!(LegacyMigrationService::detect_legacy_installation(&new_dir));
    }
}
