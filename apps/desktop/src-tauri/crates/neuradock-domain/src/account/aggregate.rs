use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;

use super::value_objects::Credentials;
use crate::shared::{AccountId, DomainError, ProviderId};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Account {
    id: AccountId,
    name: String,
    provider_id: ProviderId,
    credentials: Credentials,
    enabled: bool,
    last_check_in: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    auto_checkin_enabled: bool,
    auto_checkin_hour: u8,
    auto_checkin_minute: u8,
    last_login_at: Option<DateTime<Utc>>,
    session_token: Option<String>,
    session_expires_at: Option<DateTime<Utc>>,
    last_balance_check_at: Option<DateTime<Utc>>,
    current_balance: Option<f64>,
    total_consumed: Option<f64>,
    total_income: Option<f64>,
}

impl Account {
    pub fn new(
        name: String,
        provider_id: ProviderId,
        credentials: Credentials,
    ) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::Validation(
                "Account name cannot be empty".to_string(),
            ));
        }

        if !credentials.is_valid() {
            return Err(DomainError::InvalidCredentials(
                "Cookies are required".to_string(),
            ));
        }

        Ok(Self {
            id: AccountId::new(),
            name: name.trim().to_string(),
            provider_id,
            credentials,
            enabled: true,
            last_check_in: None,
            created_at: Utc::now(),
            auto_checkin_enabled: false,
            auto_checkin_hour: 9,
            auto_checkin_minute: 0,
            last_login_at: None,
            session_token: None,
            session_expires_at: None,
            last_balance_check_at: None,
            current_balance: None,
            total_consumed: None,
            total_income: None,
        })
    }

    pub fn restore(
        id: AccountId,
        name: String,
        provider_id: ProviderId,
        credentials: Credentials,
        enabled: bool,
        last_check_in: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        auto_checkin_enabled: bool,
        auto_checkin_hour: u8,
        auto_checkin_minute: u8,
        last_login_at: Option<DateTime<Utc>>,
        session_token: Option<String>,
        session_expires_at: Option<DateTime<Utc>>,
        last_balance_check_at: Option<DateTime<Utc>>,
        current_balance: Option<f64>,
        total_consumed: Option<f64>,
        total_income: Option<f64>,
    ) -> Self {
        Self {
            id,
            name,
            provider_id,
            credentials,
            enabled,
            last_check_in,
            created_at,
            auto_checkin_enabled,
            auto_checkin_hour,
            auto_checkin_minute,
            last_login_at,
            session_token,
            session_expires_at,
            last_balance_check_at,
            current_balance,
            total_consumed,
            total_income,
        }
    }

    pub fn id(&self) -> &AccountId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn provider_id(&self) -> &ProviderId {
        &self.provider_id
    }

    pub fn credentials(&self) -> &Credentials {
        &self.credentials
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn last_check_in(&self) -> Option<DateTime<Utc>> {
        self.last_check_in
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn update_name(&mut self, name: String) -> Result<(), DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::Validation(
                "Account name cannot be empty".to_string(),
            ));
        }
        self.name = name.trim().to_string();
        Ok(())
    }

    pub fn update_credentials(&mut self, credentials: Credentials) -> Result<(), DomainError> {
        if !credentials.is_valid() {
            return Err(DomainError::InvalidCredentials(
                "Cookies and api_user are required".to_string(),
            ));
        }
        self.credentials = credentials;
        Ok(())
    }

    pub fn toggle(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn record_check_in(&mut self) {
        self.last_check_in = Some(Utc::now());
    }

    pub fn auto_checkin_enabled(&self) -> bool {
        self.auto_checkin_enabled
    }

    pub fn auto_checkin_hour(&self) -> u8 {
        self.auto_checkin_hour
    }

    pub fn auto_checkin_minute(&self) -> u8 {
        self.auto_checkin_minute
    }

    pub fn update_auto_checkin(
        &mut self,
        enabled: bool,
        hour: u8,
        minute: u8,
    ) -> Result<(), DomainError> {
        if hour > 23 {
            return Err(DomainError::Validation(
                "Hour must be between 0 and 23".to_string(),
            ));
        }
        if minute > 59 {
            return Err(DomainError::Validation(
                "Minute must be between 0 and 59".to_string(),
            ));
        }
        self.auto_checkin_enabled = enabled;
        self.auto_checkin_hour = hour;
        self.auto_checkin_minute = minute;
        Ok(())
    }

    pub fn last_login_at(&self) -> Option<DateTime<Utc>> {
        self.last_login_at
    }

    pub fn session_token(&self) -> Option<&String> {
        self.session_token.as_ref()
    }

    pub fn session_expires_at(&self) -> Option<DateTime<Utc>> {
        self.session_expires_at
    }

    pub fn last_balance_check_at(&self) -> Option<DateTime<Utc>> {
        self.last_balance_check_at
    }

    pub fn current_balance(&self) -> Option<f64> {
        self.current_balance
    }

    pub fn total_consumed(&self) -> Option<f64> {
        self.total_consumed
    }

    pub fn total_income(&self) -> Option<f64> {
        self.total_income
    }

    pub fn update_session(&mut self, token: String, expires_at: DateTime<Utc>) {
        self.session_token = Some(token);
        self.session_expires_at = Some(expires_at);
        self.last_login_at = Some(Utc::now());
    }

    pub fn clear_session(&mut self) {
        self.session_token = None;
        self.session_expires_at = None;
    }

    pub fn is_session_valid(&self) -> bool {
        match self.session_expires_at {
            Some(expires_at) => Utc::now() < expires_at,
            None => false,
        }
    }

    pub fn update_balance(&mut self, current_balance: f64, total_consumed: f64, total_income: f64) {
        self.current_balance = Some(current_balance);
        self.total_consumed = Some(total_consumed);
        self.total_income = Some(total_income);
        self.last_balance_check_at = Some(Utc::now());
    }

    pub fn is_balance_stale(&self, max_age_hours: i64) -> bool {
        match self.last_balance_check_at {
            Some(last_check) => {
                let age = Utc::now().signed_duration_since(last_check);
                age.num_hours() > max_age_hours
            }
            None => true,
        }
    }
}
