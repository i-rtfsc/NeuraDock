use crate::application::commands::command_handler::Command;
use std::collections::HashMap;

/// Create account command
#[derive(Debug, Clone)]
pub struct CreateAccountCommand {
    pub name: String,
    pub provider_id: String,
    pub cookies: HashMap<String, String>,
    pub api_user: String,
    pub auto_checkin_enabled: Option<bool>,
    pub auto_checkin_hour: Option<u8>,
    pub auto_checkin_minute: Option<u8>,
}

impl Command for CreateAccountCommand {}

/// Create account command result
#[derive(Debug, Clone)]
pub struct CreateAccountResult {
    pub account_id: String,
}

/// Update account command
#[derive(Debug, Clone)]
pub struct UpdateAccountCommand {
    pub account_id: String,
    pub name: Option<String>,
    pub cookies: Option<HashMap<String, String>>,
    pub api_user: Option<String>,
    pub auto_checkin_enabled: Option<bool>,
    pub auto_checkin_hour: Option<u8>,
    pub auto_checkin_minute: Option<u8>,
}

impl Command for UpdateAccountCommand {}

/// Update account command result
#[derive(Debug, Clone)]
pub struct UpdateAccountResult {
    pub success: bool,
}

/// Delete account command
#[derive(Debug, Clone)]
pub struct DeleteAccountCommand {
    pub account_id: String,
}

impl Command for DeleteAccountCommand {}

/// Delete account command result
#[derive(Debug, Clone)]
pub struct DeleteAccountResult {
    pub success: bool,
}

/// Toggle account command
#[derive(Debug, Clone)]
pub struct ToggleAccountCommand {
    pub account_id: String,
    pub enabled: bool,
}

impl Command for ToggleAccountCommand {}

/// Toggle account command result
#[derive(Debug, Clone)]
pub struct ToggleAccountResult {
    pub success: bool,
}
