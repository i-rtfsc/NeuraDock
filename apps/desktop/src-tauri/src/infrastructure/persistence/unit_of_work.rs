use async_trait::async_trait;
use sqlx::{Acquire, Sqlite, SqlitePool, Transaction};
use std::sync::Arc;

use crate::domain::shared::DomainError;

/// Unit of Work pattern for managing database transactions
/// Ensures consistency across multiple repository operations
pub struct UnitOfWork<'a> {
    transaction: Transaction<'a, Sqlite>,
}

impl<'a> UnitOfWork<'a> {
    /// Begin a new transaction
    pub async fn begin(pool: &'a SqlitePool) -> Result<Self, DomainError> {
        let transaction = pool
            .begin()
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to begin transaction: {}", e)))?;

        Ok(Self { transaction })
    }

    /// Commit the transaction
    pub async fn commit(self) -> Result<(), DomainError> {
        self.transaction
            .commit()
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to commit transaction: {}", e)))
    }

    /// Rollback the transaction
    pub async fn rollback(self) -> Result<(), DomainError> {
        self.transaction
            .rollback()
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to rollback transaction: {}", e)))
    }

    /// Get a mutable reference to the transaction
    /// This allows repositories to execute queries within the transaction
    pub fn transaction(&mut self) -> &mut Transaction<'a, Sqlite> {
        &mut self.transaction
    }
}

/// Repository error mapper
/// Converts sqlx errors to domain errors with contextual information
pub struct RepositoryErrorMapper;

impl RepositoryErrorMapper {
    /// Map a sqlx error to a domain error
    pub fn map_sqlx_error(error: sqlx::Error, context: &str) -> DomainError {
        match error {
            sqlx::Error::RowNotFound => {
                DomainError::Repository(format!("{}: Entity not found", context))
            }
            sqlx::Error::Database(db_err) => {
                // Check for constraint violations
                if let Some(code) = db_err.code() {
                    if code == "2067" || code == "1555" {
                        // SQLite constraint violation codes
                        return DomainError::Validation(format!(
                            "{}: Constraint violation - {}",
                            context,
                            db_err.message()
                        ));
                    }
                }
                DomainError::Repository(format!("{}: Database error - {}", context, db_err))
            }
            sqlx::Error::Io(io_err) => {
                DomainError::Infrastructure(format!("{}: IO error - {}", context, io_err))
            }
            sqlx::Error::Tls(tls_err) => {
                DomainError::Infrastructure(format!("{}: TLS error - {}", context, tls_err))
            }
            sqlx::Error::PoolTimedOut => DomainError::Infrastructure(format!(
                "{}: Database pool timed out - too many concurrent connections",
                context
            )),
            sqlx::Error::PoolClosed => {
                DomainError::Infrastructure(format!("{}: Database pool is closed", context))
            }
            _ => DomainError::Repository(format!("{}: {}", context, error)),
        }
    }

    /// Map a serde_json error to a domain error
    pub fn map_json_error(error: serde_json::Error, context: &str) -> DomainError {
        DomainError::Repository(format!("{}: JSON serialization error - {}", context, error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_mapping_row_not_found() {
        let error = sqlx::Error::RowNotFound;
        let domain_error = RepositoryErrorMapper::map_sqlx_error(error, "Find account");
        
        match domain_error {
            DomainError::Repository(msg) => {
                assert!(msg.contains("Find account"));
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected Repository error"),
        }
    }

    #[test]
    fn test_error_mapping_pool_timeout() {
        let error = sqlx::Error::PoolTimedOut;
        let domain_error = RepositoryErrorMapper::map_sqlx_error(error, "Save account");
        
        match domain_error {
            DomainError::Infrastructure(msg) => {
                assert!(msg.contains("Save account"));
                assert!(msg.contains("timed out"));
            }
            _ => panic!("Expected Infrastructure error"),
        }
    }
}
