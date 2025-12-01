use sqlx::SqlitePool;
use neuradock_infrastructure::security::EncryptionService;
use std::path::Path;
use std::fs;
use std::sync::Arc;

pub async fn setup_in_memory_db() -> (SqlitePool, Arc<EncryptionService>) {
    // Create in-memory sqlite pool
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");

    // Discover migrations directory relative to the crate manifest
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let migrations_dir = Path::new(manifest_dir).join("migrations");

    let mut entries: Vec<_> = fs::read_dir(&migrations_dir)
        .expect("read migrations dir")
        .map(|r| r.expect("read entry"))
        .collect();

    // Sort by file name to ensure migrations run in order
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        if path.is_file() {
            let sql = fs::read_to_string(&path).expect("read migration file");
            // Execute SQL content (may contain multiple statements)
            // Split by ';' and execute non-empty statements to avoid driver issues
            for stmt in sql.split(';') {
                let s = stmt.trim();
                if s.is_empty() {
                    continue;
                }
                sqlx::query(s)
                    .execute(&pool)
                    .await
                    .expect("apply migration stmt");
            }
        }
    }

    // Create encryption service with deterministic salt for tests
    let salt = [42u8; 32];
    let encryption = Arc::new(EncryptionService::from_password("test_password", &salt)
        .expect("Create encryption service"));

    (pool, encryption)
}
