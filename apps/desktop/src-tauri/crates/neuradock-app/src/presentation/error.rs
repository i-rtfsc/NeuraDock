use neuradock_domain::shared::{DomainError, ErrorCode, ErrorSeverity};
use serde::{Deserialize, Serialize};
use specta::Type;

/// Structured error response for Tauri commands
///
/// This provides rich error information to the frontend, including:
/// - Error code for programmatic handling
/// - Human-readable message
/// - Severity level for UI presentation
/// - Recoverability flag for retry logic
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CommandError {
    /// Numeric error code (1xxx-6xxx range)
    pub code: u16,

    /// Human-readable error message
    pub message: String,

    /// Error severity level
    pub severity: ErrorSeverity,

    /// Whether the operation can be retried
    pub recoverable: bool,
}

impl CommandError {
    /// Create a new CommandError with explicit fields
    pub fn new(code: u16, message: String, severity: ErrorSeverity, recoverable: bool) -> Self {
        Self {
            code,
            message,
            severity,
            recoverable,
        }
    }

    /// Create an error from an error code and message
    pub fn from_code(error_code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: error_code.code(),
            message: message.into(),
            severity: error_code.severity(),
            recoverable: error_code.is_recoverable(),
        }
    }

    /// Create a generic infrastructure error
    pub fn infrastructure(message: impl Into<String>) -> Self {
        Self::from_code(ErrorCode::InfrastructureError, message)
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::from_code(ErrorCode::ValidationError, message)
    }

    /// Create a not found error
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::from_code(ErrorCode::AccountNotFound, message)
    }
}

impl From<DomainError> for CommandError {
    fn from(err: DomainError) -> Self {
        Self {
            code: err.code().code(),
            message: err.message().to_string(),
            severity: err.severity(),
            recoverable: err.is_recoverable(),
        }
    }
}

impl From<String> for CommandError {
    fn from(message: String) -> Self {
        Self::infrastructure(message)
    }
}

impl From<&str> for CommandError {
    fn from(message: &str) -> Self {
        Self::infrastructure(message.to_string())
    }
}

impl From<anyhow::Error> for CommandError {
    fn from(err: anyhow::Error) -> Self {
        Self::infrastructure(err.to_string())
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for CommandError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_error_from_domain_error() {
        let domain_err = DomainError::AccountNotFound("Account 123 not found".to_string());
        let cmd_err: CommandError = domain_err.into();

        assert_eq!(cmd_err.code, 2001); // AccountNotFound code
        assert_eq!(cmd_err.message, "Account 123 not found");
        assert_eq!(cmd_err.severity, ErrorSeverity::Info);
        assert_eq!(cmd_err.recoverable, false);
    }

    #[test]
    fn test_command_error_from_string() {
        let cmd_err: CommandError = "Something went wrong".into();

        assert_eq!(cmd_err.code, 5001); // InfrastructureError code
        assert_eq!(cmd_err.message, "Something went wrong");
        assert_eq!(cmd_err.severity, ErrorSeverity::Error);
    }

    #[test]
    fn test_command_error_helpers() {
        let validation_err = CommandError::validation("Invalid input");
        assert_eq!(validation_err.code, 6001);

        let not_found_err = CommandError::not_found("Resource not found");
        assert_eq!(not_found_err.code, 2001);

        let infra_err = CommandError::infrastructure("Service unavailable");
        assert_eq!(infra_err.code, 5001);
    }
}
