use chrono::{DateTime, Utc};
use serde_json;
use sqlx::FromRow;

use crate::persistence::RepositoryErrorMapper;
use crate::security::EncryptionService;
use neuradock_domain::account::{Account, Credentials};
use neuradock_domain::shared::{AccountId, DomainError, ProviderId};

#[derive(FromRow)]
pub(super) struct AccountRow {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    pub cookies: String,
    pub api_user: String,
    pub enabled: bool,
    pub last_check_in: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub auto_checkin_enabled: bool,
    pub auto_checkin_hour: i64,
    pub auto_checkin_minute: i64,
    pub last_login_at: Option<DateTime<Utc>>,
    pub session_token: Option<String>,
    pub session_expires_at: Option<DateTime<Utc>>,
    pub last_balance_check_at: Option<DateTime<Utc>>,
    pub current_balance: Option<f64>,
    pub total_consumed: Option<f64>,
    pub total_income: Option<f64>,
}

impl AccountRow {
    pub fn to_account(self, encryption: &EncryptionService) -> Result<Account, DomainError> {
        // Decrypt cookies JSON - fail if decryption fails (no fallback to plaintext)
        // This ensures all sensitive data is properly encrypted
        let cookies_json = encryption.decrypt(&self.cookies)
            .map_err(|e| {
                DomainError::DataIntegrity(format!(
                    "Failed to decrypt cookies for account {}: {}. Data may be corrupted or using wrong encryption key.",
                    self.id, e
                ))
            })?;

        let cookies = serde_json::from_str(&cookies_json)
            .map_err(|e| RepositoryErrorMapper::map_json_error(e, "Deserialize account cookies"))?;

        // Decrypt API user - fail if decryption fails (no fallback to plaintext)
        let api_user = encryption.decrypt(&self.api_user)
            .map_err(|e| {
                DomainError::DataIntegrity(format!(
                    "Failed to decrypt api_user for account {}: {}. Data may be corrupted or using wrong encryption key.",
                    self.id, e
                ))
            })?;

        let credentials = Credentials::new(cookies, api_user);

        Ok(Account::builder(
            AccountId::from_string(&self.id),
            self.name,
            ProviderId::from_string(&self.provider_id),
            credentials,
        )
        .enabled(self.enabled)
        .last_check_in(self.last_check_in)
        .created_at(self.created_at)
        .auto_checkin_enabled(self.auto_checkin_enabled)
        .auto_checkin_hour(self.auto_checkin_hour as u8)
        .auto_checkin_minute(self.auto_checkin_minute as u8)
        .last_login_at(self.last_login_at)
        .session_token(self.session_token)
        .session_expires_at(self.session_expires_at)
        .last_balance_check_at(self.last_balance_check_at)
        .current_balance(self.current_balance)
        .total_consumed(self.total_consumed)
        .total_income(self.total_income)
        .build())
    }
}
