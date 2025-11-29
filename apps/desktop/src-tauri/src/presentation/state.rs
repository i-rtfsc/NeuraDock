use sqlx::SqlitePool;
use std::sync::Arc;
use tauri::Manager;

use crate::application::queries::CheckInStreakQueries;
use crate::application::services::AutoCheckInScheduler;
use crate::domain::account::AccountRepository;
use crate::infrastructure::persistence::{repositories::SqliteAccountRepository, Database};

pub struct AppState {
    pub pool: Arc<SqlitePool>,
    pub db: Arc<Database>,
    pub account_repo: Arc<dyn AccountRepository>,
    pub scheduler: Arc<AutoCheckInScheduler>,
    pub streak_queries: Arc<CheckInStreakQueries>,
    pub app_handle: tauri::AppHandle,
}

impl AppState {
    pub async fn new(app_handle: tauri::AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        // Get app data directory (~/Library/Application Support/com.neuradock.app/)
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data directory: {}", e))?;

        // Create directory if it doesn't exist
        std::fs::create_dir_all(&app_data_dir)
            .map_err(|e| format!("Failed to create app data directory: {}", e))?;

        // let db_filename = if cfg!(debug_assertions) {
        //     "neuradock.db"
        // } else {
        //     "neuradock.db"
        // };
        let db_filename = "neuradock.db";

        let db_path = app_data_dir.join(db_filename);
        let db_path_str = db_path.to_str().ok_or("Invalid database path")?;

        eprintln!("Database path: {}", db_path_str);

        eprintln!("🔌 Connecting to database...");
        let database = Database::new(db_path_str).await?;
        eprintln!("✓ Database connection established");

        eprintln!("🔄 Running migrations...");
        database.run_migrations().await?;
        eprintln!("✓ Migrations completed");

        let pool = Arc::new(database.pool().clone());
        let db = Arc::new(database);
        let account_repo =
            Arc::new(SqliteAccountRepository::new(pool.clone())) as Arc<dyn AccountRepository>;
        let streak_queries = Arc::new(CheckInStreakQueries::new(pool.clone()));

        eprintln!("📊 Initializing scheduler...");
        // Initialize scheduler
        let scheduler = Arc::new(AutoCheckInScheduler::new(account_repo.clone()).await?);
        eprintln!("✓ Scheduler initialized");

        eprintln!("▶️  Starting scheduler...");
        scheduler.start().await?;
        eprintln!("✓ Scheduler started");

        // Load existing schedules from database
        eprintln!("📋 Loading auto check-in schedules...");
        use crate::presentation::commands::get_builtin_providers;
        let providers = get_builtin_providers();
        eprintln!("📦 Got {} providers", providers.len());

        eprintln!("🔍 Calling reload_schedules...");
        if let Err(e) = scheduler
            .reload_schedules(providers, account_repo.clone(), app_handle.clone())
            .await
        {
            eprintln!("⚠️  Failed to load schedules: {}", e);
        } else {
            eprintln!("✓ Auto check-in schedules loaded successfully");
        }

        Ok(Self {
            pool,
            db,
            account_repo,
            scheduler,
            streak_queries,
            app_handle,
        })
    }
}
