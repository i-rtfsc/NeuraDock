use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;

use neuradock_domain::account::Account;

use super::BalanceDto;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AccountDto {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    pub provider_name: String,
    pub enabled: bool,
    pub last_check_in: Option<String>,
    pub created_at: String,
    pub auto_checkin_enabled: bool,
    pub auto_checkin_hour: u8,
    pub auto_checkin_minute: u8,
    pub check_in_interval_hours: u8,
    pub last_balance_check_at: Option<String>,
    pub current_balance: Option<f64>,
    pub total_consumed: Option<f64>,
    pub total_income: Option<f64>,
    pub is_balance_stale: bool,
    pub is_online: bool,
    // Session expiration info for frontend display
    pub session_expires_at: Option<String>,
    pub session_expires_soon: bool, // true if session expires within 7 days
    pub session_days_remaining: Option<i64>, // days until session expires
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AccountDetailDto {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    pub provider_name: String,
    pub api_user: String,
    pub cookies: HashMap<String, String>,
    pub cookies_count: i32,
    pub enabled: bool,
    pub last_check_in: Option<String>,
    pub last_balance: Option<BalanceDto>,
    pub created_at: String,
    pub auto_checkin_enabled: bool,
    pub auto_checkin_hour: u8,
    pub auto_checkin_minute: u8,
    pub check_in_interval_hours: u8,
}

// ============================================================
// Account DTO Conversions
// ============================================================

/// Helper struct for Account -> AccountDto conversion
/// Provides provider name which is not part of the domain model
pub struct AccountDtoMapper<'a> {
    pub provider_name: String,
    pub now: DateTime<Utc>,
    account: &'a Account,
}

impl<'a> AccountDtoMapper<'a> {
    pub fn new(account: &'a Account, provider_name: String) -> Self {
        Self {
            provider_name,
            now: Utc::now(),
            account,
        }
    }

    pub fn with_time(mut self, now: DateTime<Utc>) -> Self {
        self.now = now;
        self
    }

    pub fn to_dto(self) -> AccountDto {
        let acc = self.account;

        // Check if balance is stale (> 24 hours old)
        let is_balance_stale = acc.is_balance_stale(24);

        // Consider account "online" if session is valid OR balance check is recent
        let is_online = acc.is_session_valid() || !is_balance_stale;

        // Calculate session expiration info
        let session_expires_at = acc.session_expires_at();
        let (session_expires_soon, session_days_remaining) = match session_expires_at {
            Some(expires_at) => {
                let duration = expires_at.signed_duration_since(self.now);
                let days_remaining = duration.num_days();
                let expires_soon = days_remaining <= 7;
                (expires_soon, Some(days_remaining.max(0)))
            }
            None => (false, None),
        };

        AccountDto {
            id: acc.id().as_str().to_string(),
            name: acc.name().to_string(),
            provider_id: acc.provider_id().as_str().to_string(),
            provider_name: self.provider_name,
            enabled: acc.is_enabled(),
            last_check_in: acc.last_check_in().map(|dt| dt.to_rfc3339()),
            created_at: acc.created_at().to_rfc3339(),
            auto_checkin_enabled: acc.auto_checkin_enabled(),
            auto_checkin_hour: acc.auto_checkin_hour(),
            auto_checkin_minute: acc.auto_checkin_minute(),
            check_in_interval_hours: acc.check_in_interval_hours(),
            last_balance_check_at: acc.last_balance_check_at().map(|dt| dt.to_rfc3339()),
            current_balance: acc.current_balance(),
            total_consumed: acc.total_consumed(),
            total_income: acc.total_income(),
            is_balance_stale,
            is_online,
            session_expires_at: session_expires_at.map(|dt| dt.to_rfc3339()),
            session_expires_soon,
            session_days_remaining,
        }
    }
}

/// Helper for AccountDetailDto conversion
pub struct AccountDetailDtoMapper<'a> {
    pub provider_name: String,
    account: &'a Account,
    pub last_balance: Option<BalanceDto>,
}

impl<'a> AccountDetailDtoMapper<'a> {
    pub fn new(account: &'a Account, provider_name: String) -> Self {
        Self {
            provider_name,
            account,
            last_balance: None,
        }
    }

    pub fn with_balance(mut self, balance: Option<BalanceDto>) -> Self {
        self.last_balance = balance;
        self
    }

    pub fn to_dto(self) -> AccountDetailDto {
        let acc = self.account;

        AccountDetailDto {
            id: acc.id().as_str().to_string(),
            name: acc.name().to_string(),
            provider_id: acc.provider_id().as_str().to_string(),
            provider_name: self.provider_name,
            api_user: acc.credentials().api_user().to_string(),
            cookies: acc.credentials().cookies().clone(),
            cookies_count: acc.credentials().cookies().len() as i32,
            enabled: acc.is_enabled(),
            last_check_in: acc.last_check_in().map(|dt| dt.to_rfc3339()),
            last_balance: self.last_balance,
            created_at: acc.created_at().to_rfc3339(),
            auto_checkin_enabled: acc.auto_checkin_enabled(),
            auto_checkin_hour: acc.auto_checkin_hour(),
            auto_checkin_minute: acc.auto_checkin_minute(),
            check_in_interval_hours: acc.check_in_interval_hours(),
        }
    }
}

// ============================================================
// Account Input DTOs
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CreateAccountInput {
    pub name: String,
    pub provider_id: String,
    pub cookies: HashMap<String, String>,
    pub api_user: String,
    pub auto_checkin_enabled: Option<bool>,
    pub auto_checkin_hour: Option<u8>,
    pub auto_checkin_minute: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UpdateAccountInput {
    pub account_id: String,
    pub name: Option<String>,
    pub cookies: Option<HashMap<String, String>>,
    pub api_user: Option<String>,
    pub auto_checkin_enabled: Option<bool>,
    pub auto_checkin_hour: Option<u8>,
    pub auto_checkin_minute: Option<u8>,
    pub check_in_interval_hours: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ImportAccountInput {
    pub name: String,
    pub provider: String,
    pub cookies: HashMap<String, String>,
    pub api_user: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BatchImportResult {
    pub total: i32,
    pub succeeded: i32,
    pub failed: i32,
    pub results: Vec<ImportItemResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ImportItemResult {
    pub success: bool,
    pub account_id: Option<String>,
    pub account_name: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BatchUpdateResult {
    pub total: i32,
    pub updated: i32,
    pub created: i32,
    pub failed: i32,
    pub results: Vec<UpdateItemResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UpdateItemResult {
    pub success: bool,
    pub account_id: Option<String>,
    pub account_name: String,
    pub action: String, // "updated", "created", "failed"
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ExportAccountsInput {
    pub account_ids: Vec<String>,
    pub include_credentials: bool,
}
