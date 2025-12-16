use crate::application::commands::command_handler::Command;
use serde::{Deserialize, Serialize};
use specta::Type;

/// Create provider command
#[derive(Debug, Clone, Deserialize, Type)]
pub struct CreateProviderCommand {
    pub name: String,
    pub domain: String,
    pub needs_waf_bypass: bool,
    // Optional API paths (with defaults)
    pub login_path: Option<String>,
    pub sign_in_path: Option<String>,
    pub user_info_path: Option<String>,
    pub token_api_path: Option<String>,
    pub models_path: Option<String>,
    pub api_user_key: Option<String>,
}

impl Command for CreateProviderCommand {}

/// Create provider command result
#[derive(Debug, Clone, Serialize, Type)]
pub struct CreateProviderResult {
    pub provider_id: String,
}

/// Update provider command
#[derive(Debug, Clone, Deserialize, Type)]
pub struct UpdateProviderCommand {
    pub provider_id: String,
    pub name: Option<String>,
    pub domain: Option<String>,
    pub needs_waf_bypass: Option<bool>,
    // Optional API paths
    pub login_path: Option<String>,
    pub sign_in_path: Option<String>,
    pub user_info_path: Option<String>,
    pub token_api_path: Option<String>,
    pub models_path: Option<String>,
    pub api_user_key: Option<String>,
}

impl Command for UpdateProviderCommand {}

/// Update provider command result
#[derive(Debug, Clone, Serialize, Type)]
pub struct UpdateProviderResult {
    pub success: bool,
}

/// Delete provider command
#[derive(Debug, Clone, Deserialize, Type)]
pub struct DeleteProviderCommand {
    pub provider_id: String,
}

impl Command for DeleteProviderCommand {}

/// Delete provider command result
#[derive(Debug, Clone, Serialize, Type)]
pub struct DeleteProviderResult {
    pub success: bool,
}
