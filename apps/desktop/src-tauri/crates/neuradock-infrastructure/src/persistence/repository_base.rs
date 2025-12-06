use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;

use neuradock_domain::shared::DomainError;
use crate::persistence::ResultExt;

/// Base struct for SQLite repositories
/// 
/// Provides common functionality:
/// - Pool management
/// - Error mapping
/// - Transaction support
/// - Common query patterns
pub struct SqliteRepositoryBase {
    pool: Arc<SqlitePool>,
}

impl SqliteRepositoryBase {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
    
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
    
    pub fn arc_pool(&self) -> Arc<SqlitePool> {
        Arc::clone(&self.pool)
    }

    /// Execute a query that returns a single optional row
    pub async fn fetch_optional<'q, T>(
        &self,
        query: sqlx::query::QueryAs<'q, sqlx::Sqlite, T, sqlx::sqlite::SqliteArguments<'q>>,
        operation: &str,
    ) -> Result<Option<T>, DomainError>
    where
        T: Send + Unpin + for<'r> FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        query
            .fetch_optional(self.pool())
            .await
            .map_repo_error(operation)
    }

    /// Execute a query that returns a single row (error if not found)
    pub async fn fetch_one<'q, T>(
        &self,
        query: sqlx::query::QueryAs<'q, sqlx::Sqlite, T, sqlx::sqlite::SqliteArguments<'q>>,
        operation: &str,
    ) -> Result<T, DomainError>
    where
        T: Send + Unpin + for<'r> FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        query
            .fetch_one(self.pool())
            .await
            .map_repo_error(operation)
    }

    /// Execute a query that returns multiple rows
    pub async fn fetch_all<'q, T>(
        &self,
        query: sqlx::query::QueryAs<'q, sqlx::Sqlite, T, sqlx::sqlite::SqliteArguments<'q>>,
        operation: &str,
    ) -> Result<Vec<T>, DomainError>
    where
        T: Send + Unpin + for<'r> FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        query
            .fetch_all(self.pool())
            .await
            .map_repo_error(operation)
    }

    /// Execute a non-query statement (INSERT, UPDATE, DELETE)
    pub async fn execute<'q>(
        &self,
        query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
        operation: &str,
    ) -> Result<u64, DomainError> {
        let result = query
            .execute(self.pool())
            .await
            .map_repo_error(operation)?;
        
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn test_repository_base_creation() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(":memory:")
            .await
            .unwrap();
        
        let pool = Arc::new(pool);
        let repo = SqliteRepositoryBase::new(pool.clone());
        
        assert!(Arc::ptr_eq(&pool, &repo.arc_pool()));
    }
}
