use crate::persistence::RepositoryErrorMapper;
use neuradock_domain::shared::DomainError;

/// Extension trait for Result to simplify error mapping in repositories
///
/// This trait reduces boilerplate code by providing a convenient method
/// to map various error types to DomainError with contextual information.
///
/// # Example
/// ```
/// use result_ext::ResultExt;
///
/// let result = sqlx::query("SELECT * FROM users")
///     .fetch_one(&pool)
///     .await
///     .map_repo_error("Fetch user")?;
/// ```
pub trait ResultExt<T> {
    /// Map an error to a DomainError with context
    ///
    /// This is a convenience method that wraps RepositoryErrorMapper
    /// to reduce verbose `.map_err(|e| RepositoryErrorMapper::...)` calls.
    fn map_repo_error(self, context: &str) -> Result<T, DomainError>;
}

impl<T> ResultExt<T> for Result<T, sqlx::Error> {
    fn map_repo_error(self, context: &str) -> Result<T, DomainError> {
        self.map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, context))
    }
}

impl<T> ResultExt<T> for Result<T, serde_json::Error> {
    fn map_repo_error(self, context: &str) -> Result<T, DomainError> {
        self.map_err(|e| RepositoryErrorMapper::map_json_error(e, context))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_ext_with_json_error() {
        let invalid_json = "{invalid}";
        let result: Result<serde_json::Value, _> = serde_json::from_str(invalid_json);

        let domain_error = result.map_repo_error("Parse config");
        assert!(domain_error.is_err());

        match domain_error.unwrap_err() {
            DomainError::Repository(msg) => {
                assert!(msg.contains("Parse config"));
                assert!(msg.contains("JSON"));
            }
            _ => panic!("Expected Repository error"),
        }
    }
}
