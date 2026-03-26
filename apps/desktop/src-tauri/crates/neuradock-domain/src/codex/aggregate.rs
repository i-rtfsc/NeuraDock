use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::shared::DomainError;

// ─── ID ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
pub struct CodexAccountId(String);

impl CodexAccountId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(s: &str) -> Self {
        Self(s.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for CodexAccountId {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Value Objects ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CodexAccountStatus {
    Active,
    Expired,
    Banned,
}

impl CodexAccountStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Active => "active",
            Self::Expired => "expired",
            Self::Banned => "banned",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "expired" => Self::Expired,
            "banned" => Self::Banned,
            _ => Self::Active,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CodexAccountSource {
    Register,
    Import,
}

impl CodexAccountSource {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Register => "register",
            Self::Import => "import",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "import" => Self::Import,
            _ => Self::Register,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CodexRateLimitWindow {
    used_percent: f64,
    window_minutes: Option<i64>,
    resets_at: Option<DateTime<Utc>>,
}

impl CodexRateLimitWindow {
    pub fn new(used_percent: f64, window_minutes: Option<i64>, resets_at: Option<DateTime<Utc>>) -> Self {
        Self {
            used_percent,
            window_minutes,
            resets_at,
        }
    }

    pub fn used_percent(&self) -> f64 { self.used_percent }
    pub fn window_minutes(&self) -> Option<i64> { self.window_minutes }
    pub fn resets_at(&self) -> Option<DateTime<Utc>> { self.resets_at }
}

// ─── Aggregate ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CodexAccount {
    id: CodexAccountId,
    email: String,
    password: Option<String>,

    // OAuth tokens
    access_token: Option<String>,
    refresh_token: Option<String>,
    id_token: Option<String>,
    account_id: Option<String>, // OpenAI org/workspace ID
    web_session_cookie: Option<String>,
    web_session_device_id: Option<String>,

    // Plan & quota (cached)
    plan_type: Option<String>,
    has_credits: Option<bool>,
    is_unlimited: Option<bool>,
    credit_balance: Option<String>,
    primary_window: Option<CodexRateLimitWindow>,
    secondary_window: Option<CodexRateLimitWindow>,
    quota_checked_at: Option<DateTime<Utc>>,

    // Token expiry
    token_expires_at: Option<DateTime<Utc>>,

    // Metadata
    last_refresh_at: Option<DateTime<Utc>>,
    tempmail_token: Option<String>,
    source: CodexAccountSource,
    status: CodexAccountStatus,

    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl CodexAccount {
    pub fn new(email: String, password: Option<String>, source: CodexAccountSource) -> Result<Self, DomainError> {
        if email.trim().is_empty() {
            return Err(DomainError::Validation("Email cannot be empty".to_string()));
        }
        let now = Utc::now();
        Ok(Self {
            id: CodexAccountId::new(),
            email: email.trim().to_lowercase(),
            password,
            access_token: None,
            refresh_token: None,
            id_token: None,
            account_id: None,
            web_session_cookie: None,
            web_session_device_id: None,
            plan_type: None,
            has_credits: None,
            is_unlimited: None,
            credit_balance: None,
            primary_window: None,
            secondary_window: None,
            quota_checked_at: None,
            token_expires_at: None,
            last_refresh_at: None,
            tempmail_token: None,
            source,
            status: CodexAccountStatus::Active,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn restore(
        id: CodexAccountId,
        email: String,
        password: Option<String>,
        access_token: Option<String>,
        refresh_token: Option<String>,
        id_token: Option<String>,
        account_id: Option<String>,
        web_session_cookie: Option<String>,
        web_session_device_id: Option<String>,
        plan_type: Option<String>,
        has_credits: Option<bool>,
        is_unlimited: Option<bool>,
        credit_balance: Option<String>,
        primary_window: Option<CodexRateLimitWindow>,
        secondary_window: Option<CodexRateLimitWindow>,
        quota_checked_at: Option<DateTime<Utc>>,
        token_expires_at: Option<DateTime<Utc>>,
        last_refresh_at: Option<DateTime<Utc>>,
        tempmail_token: Option<String>,
        source: CodexAccountSource,
        status: CodexAccountStatus,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            email,
            password,
            access_token,
            refresh_token,
            id_token,
            account_id,
            web_session_cookie,
            web_session_device_id,
            plan_type,
            has_credits,
            is_unlimited,
            credit_balance,
            primary_window,
            secondary_window,
            quota_checked_at,
            token_expires_at,
            last_refresh_at,
            tempmail_token,
            source,
            status,
            created_at,
            updated_at,
        }
    }

    // ── Domain methods ─────────────────────────────────────────────────────

    pub fn apply_tokens(
        &mut self,
        access_token: String,
        refresh_token: String,
        id_token: String,
        account_id: Option<String>,
        token_expires_at: Option<DateTime<Utc>>,
    ) {
        self.access_token = Some(access_token);
        self.refresh_token = Some(refresh_token);
        self.id_token = Some(id_token);
        if let Some(aid) = account_id {
            self.account_id = Some(aid);
        }
        self.token_expires_at = token_expires_at;
        self.last_refresh_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn set_payment_session(
        &mut self,
        web_session_cookie: String,
        web_session_device_id: Option<String>,
    ) {
        self.web_session_cookie = Some(web_session_cookie);
        self.web_session_device_id = web_session_device_id;
        self.updated_at = Utc::now();
    }

    pub fn apply_quota(
        &mut self,
        plan_type: Option<String>,
        has_credits: Option<bool>,
        is_unlimited: Option<bool>,
        credit_balance: Option<String>,
        primary_window: Option<CodexRateLimitWindow>,
        secondary_window: Option<CodexRateLimitWindow>,
    ) {
        self.plan_type = plan_type;
        self.has_credits = has_credits;
        self.is_unlimited = is_unlimited;
        self.credit_balance = credit_balance;
        self.primary_window = primary_window;
        self.secondary_window = secondary_window;
        self.quota_checked_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn set_tempmail_token(&mut self, token: String) {
        self.tempmail_token = Some(token);
        self.updated_at = Utc::now();
    }

    pub fn has_valid_tokens(&self) -> bool {
        self.access_token.is_some() && self.refresh_token.is_some()
    }

    pub fn is_token_expired(&self) -> bool {
        match &self.token_expires_at {
            Some(exp) => Utc::now() > *exp,
            None => false,
        }
    }

    pub fn token_days_remaining(&self) -> Option<i64> {
        self.token_expires_at.map(|exp| {
            let remaining = exp - Utc::now();
            remaining.num_days()
        })
    }

    // ── Getters ────────────────────────────────────────────────────────────

    pub fn id(&self) -> &CodexAccountId { &self.id }
    pub fn email(&self) -> &str { &self.email }
    pub fn password(&self) -> Option<&str> { self.password.as_deref() }
    pub fn access_token(&self) -> Option<&str> { self.access_token.as_deref() }
    pub fn refresh_token(&self) -> Option<&str> { self.refresh_token.as_deref() }
    pub fn id_token(&self) -> Option<&str> { self.id_token.as_deref() }
    pub fn account_id(&self) -> Option<&str> { self.account_id.as_deref() }
    pub fn web_session_cookie(&self) -> Option<&str> { self.web_session_cookie.as_deref() }
    pub fn web_session_device_id(&self) -> Option<&str> { self.web_session_device_id.as_deref() }
    pub fn has_payment_session(&self) -> bool {
        self.web_session_cookie
            .as_ref()
            .map(|cookie| !cookie.trim().is_empty())
            .unwrap_or(false)
    }
    pub fn plan_type(&self) -> Option<&str> { self.plan_type.as_deref() }
    pub fn has_credits(&self) -> Option<bool> { self.has_credits }
    pub fn is_unlimited(&self) -> Option<bool> { self.is_unlimited }
    pub fn credit_balance(&self) -> Option<&str> { self.credit_balance.as_deref() }
    pub fn primary_window(&self) -> Option<&CodexRateLimitWindow> { self.primary_window.as_ref() }
    pub fn secondary_window(&self) -> Option<&CodexRateLimitWindow> { self.secondary_window.as_ref() }
    pub fn quota_checked_at(&self) -> Option<DateTime<Utc>> { self.quota_checked_at }
    pub fn token_expires_at(&self) -> Option<DateTime<Utc>> { self.token_expires_at }
    pub fn last_refresh_at(&self) -> Option<DateTime<Utc>> { self.last_refresh_at }
    pub fn tempmail_token(&self) -> Option<&str> { self.tempmail_token.as_deref() }
    pub fn source(&self) -> &CodexAccountSource { &self.source }
    pub fn status(&self) -> &CodexAccountStatus { &self.status }
    pub fn created_at(&self) -> DateTime<Utc> { self.created_at }
    pub fn updated_at(&self) -> DateTime<Utc> { self.updated_at }
}
