use neuradock_domain::shared::DomainError;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::fs::OpenOptions;
use std::path::Path;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(db_path: &str) -> Result<Self, DomainError> {
        let path = Path::new(db_path);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DomainError::Infrastructure(format!("Failed to create DB directory: {}", e))
            })?;
        }

        if !path.exists() {
            OpenOptions::new()
                .create(true)
                .write(true)
                .open(path)
                .map_err(|e| {
                    DomainError::Infrastructure(format!("Failed to create DB file: {}", e))
                })?;
        }

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&format!("sqlite:{}", db_path))
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;

        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> Result<(), DomainError> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;
        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
