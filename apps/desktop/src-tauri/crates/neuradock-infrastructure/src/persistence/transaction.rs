use async_trait::async_trait;
use neuradock_domain::shared::transaction::{TransactionContext, UnitOfWork, UnitOfWorkError};
use sqlx::{Pool, Sqlite, Transaction as SqlxTransaction};
use std::sync::Arc;

/// Sqlite implementation of TransactionContext
/// Holds a reference to the pool to ensure it outlives the transaction
pub struct SqliteTransactionContext<'a> {
    tx: Option<SqlxTransaction<'a, Sqlite>>,
    _pool: Arc<Pool<Sqlite>>, // Keep pool alive during transaction
}

impl<'a> SqliteTransactionContext<'a> {
    pub fn new(tx: SqlxTransaction<'a, Sqlite>, pool: Arc<Pool<Sqlite>>) -> Self {
        Self {
            tx: Some(tx),
            _pool: pool,
        }
    }

    /// Get mutable reference to the underlying transaction
    /// This is used by repositories to execute queries within the transaction
    pub fn inner_mut(&mut self) -> &mut SqlxTransaction<'a, Sqlite> {
        self.tx.as_mut().expect("Transaction already consumed")
    }
}

#[async_trait]
impl<'a> TransactionContext for SqliteTransactionContext<'a> {
    type Error = sqlx::Error;

    async fn commit(mut self: Box<Self>) -> Result<(), Self::Error> {
        if let Some(tx) = self.tx.take() {
            tx.commit().await
        } else {
            Err(sqlx::Error::PoolClosed)
        }
    }

    async fn rollback(mut self: Box<Self>) -> Result<(), Self::Error> {
        if let Some(tx) = self.tx.take() {
            tx.rollback().await
        } else {
            Err(sqlx::Error::PoolClosed)
        }
    }
}

/// Sqlite implementation of Unit of Work
pub struct SqliteUnitOfWork {
    pool: Arc<Pool<Sqlite>>,
}

impl SqliteUnitOfWork {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UnitOfWork for SqliteUnitOfWork {
    type Transaction = SqliteTransactionContext<'static>;

    async fn begin(&self) -> Result<Box<Self::Transaction>, UnitOfWorkError> {
        let tx = self
            .pool
            .begin()
            .await
            .map_err(|e| UnitOfWorkError::TransactionFailed(e.to_string()))?;

        // SAFETY: We're converting the transaction to 'static lifetime.
        // This is now safe because SqliteTransactionContext holds an Arc<Pool>
        // reference, ensuring the pool outlives the transaction.
        let static_tx: SqlxTransaction<'static, Sqlite> = unsafe { std::mem::transmute(tx) };

        Ok(Box::new(SqliteTransactionContext::new(
            static_tx,
            Arc::clone(&self.pool),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    #[sqlx::test]
    async fn test_unit_of_work_commit() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let uow = SqliteUnitOfWork::new(Arc::new(pool));

        let tx = uow.begin().await.unwrap();
        tx.commit().await.unwrap();
    }

    #[sqlx::test]
    async fn test_unit_of_work_rollback() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let uow = SqliteUnitOfWork::new(Arc::new(pool));

        let tx = uow.begin().await.unwrap();
        tx.rollback().await.unwrap();
    }

    /// Test that transaction holds pool reference and prevents premature drop
    #[sqlx::test]
    async fn test_transaction_safety_with_pool_reference() {
        let pool = Arc::new(SqlitePool::connect(":memory:").await.unwrap());
        let uow = SqliteUnitOfWork::new(Arc::clone(&pool));

        // Begin transaction - it should hold a reference to the pool
        let tx = uow.begin().await.unwrap();

        // Drop the original pool Arc - transaction should still be valid
        // because SqliteTransactionContext holds its own Arc<Pool> reference
        drop(pool);

        // Transaction should still work
        tx.commit().await.unwrap();
    }

    /// Test that multiple concurrent transactions can coexist
    #[sqlx::test]
    async fn test_concurrent_transactions() {
        let pool = Arc::new(SqlitePool::connect(":memory:").await.unwrap());
        let uow = SqliteUnitOfWork::new(Arc::clone(&pool));

        let tx1 = uow.begin().await.unwrap();
        let tx2 = uow.begin().await.unwrap();

        // Both transactions should be independent
        tx1.commit().await.unwrap();
        tx2.rollback().await.unwrap();
    }
}
