use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LegacyImportFilesDto {
    pub production_db: bool,
    pub development_db: bool,
    pub encryption_salt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LegacyImportResultDto {
    pub legacy_dir: String,
    pub new_dir: String,
    pub staged: bool,
    pub already_staged: bool,
    pub files: LegacyImportFilesDto,
    pub requires_restart: bool,
}
