use serde::{Deserialize, Serialize};
use specta::Type;
use uuid::Uuid;

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
        pub struct $name(String);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4().to_string())
            }

            pub fn from_string(s: &str) -> Self {
                Self(s.to_string())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

define_id!(AccountId);
define_id!(ProviderId);
define_id!(JobId);
define_id!(ChannelId);
define_id!(StreakId);

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Invalid credentials: {0}")]
    InvalidCredentials(String),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("Check-in failed: {0}")]
    CheckInFailed(String),

    #[error("Repository error: {0}")]
    Repository(String),

    #[error("Infrastructure error: {0}")]
    Infrastructure(String),

    #[error("Validation error: {0}")]
    Validation(String),
}
