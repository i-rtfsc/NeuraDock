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

/// Error codes for structured error handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum ErrorCode {
    // Authentication & Authorization (1xxx)
    InvalidCredentials = 1001,
    ExpiredSession = 1002,
    MissingApiKey = 1003,
    
    // Resource Not Found (2xxx)
    AccountNotFound = 2001,
    ProviderNotFound = 2002,
    SessionNotFound = 2003,
    
    // Business Logic (3xxx)
    CheckInFailed = 3001,
    CheckInTooFrequent = 3002,
    AccountDisabled = 3003,
    InvalidProviderConfig = 3004,
    
    // Data & Persistence (4xxx)
    RepositoryError = 4001,
    DatabaseConstraintViolation = 4002,
    DataIntegrityError = 4003,
    SerializationError = 4004,
    EncryptionError = 4005,
    DecryptionError = 4006,
    
    // Infrastructure (5xxx)
    InfrastructureError = 5001,
    NetworkError = 5002,
    TimeoutError = 5003,
    ExternalServiceError = 5004,
    
    // Validation (6xxx)
    ValidationError = 6001,
    InvalidInput = 6002,
    MissingRequiredField = 6003,
}

impl ErrorCode {
    /// Get error code as integer
    pub fn code(&self) -> u16 {
        *self as u16
    }
    
    /// Get error severity
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            ErrorCode::InvalidCredentials
            | ErrorCode::ExpiredSession
            | ErrorCode::CheckInFailed
            | ErrorCode::NetworkError => ErrorSeverity::Warning,
            
            ErrorCode::AccountNotFound
            | ErrorCode::ProviderNotFound
            | ErrorCode::ValidationError
            | ErrorCode::InvalidInput => ErrorSeverity::Info,
            
            ErrorCode::DataIntegrityError
            | ErrorCode::DatabaseConstraintViolation
            | ErrorCode::EncryptionError
            | ErrorCode::DecryptionError
            | ErrorCode::InfrastructureError => ErrorSeverity::Error,
            
            _ => ErrorSeverity::Warning,
        }
    }
    
    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            ErrorCode::NetworkError
            | ErrorCode::TimeoutError
            | ErrorCode::ExternalServiceError
            | ErrorCode::CheckInFailed
        )
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

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
    
    #[error("Data integrity error: {0}")]
    DataIntegrity(String),
}

impl DomainError {
    /// Get error code
    pub fn code(&self) -> ErrorCode {
        match self {
            DomainError::InvalidCredentials(_) => ErrorCode::InvalidCredentials,
            DomainError::AccountNotFound(_) => ErrorCode::AccountNotFound,
            DomainError::ProviderNotFound(_) => ErrorCode::ProviderNotFound,
            DomainError::CheckInFailed(_) => ErrorCode::CheckInFailed,
            DomainError::Repository(_) => ErrorCode::RepositoryError,
            DomainError::Infrastructure(_) => ErrorCode::InfrastructureError,
            DomainError::Validation(_) => ErrorCode::ValidationError,
            DomainError::DataIntegrity(_) => ErrorCode::DataIntegrityError,
        }
    }
    
    /// Get error message
    pub fn message(&self) -> &str {
        match self {
            DomainError::InvalidCredentials(msg)
            | DomainError::AccountNotFound(msg)
            | DomainError::ProviderNotFound(msg)
            | DomainError::CheckInFailed(msg)
            | DomainError::Repository(msg)
            | DomainError::Infrastructure(msg)
            | DomainError::Validation(msg)
            | DomainError::DataIntegrity(msg) => msg,
        }
    }
    
    /// Get error severity
    pub fn severity(&self) -> ErrorSeverity {
        self.code().severity()
    }
    
    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        self.code().is_recoverable()
    }
    
    /// Format error with code
    pub fn format_with_code(&self) -> String {
        format!("[{}] {}", self.code().code(), self)
    }
}
