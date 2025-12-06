use sqlx::SqlitePool;
use std::sync::Arc;

/// Base struct for SQLite repositories
/// 
/// Provides common functionality:
/// - Pool management
/// - Error mapping
/// - Transaction support
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
