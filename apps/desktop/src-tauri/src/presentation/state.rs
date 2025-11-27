use std::sync::Arc;
use sqlx::SqlitePool;
use tauri::Manager;

use crate::domain::account::AccountRepository;
use crate::infrastructure::persistence::{Database, repositories::SqliteAccountRepository};
use crate::application::services::AutoCheckInScheduler;

pub struct AppState {
    pub pool: Arc<SqlitePool>,
    pub db: Arc<Database>,
    pub account_repo: Arc<dyn AccountRepository>,
    pub scheduler: Arc<AutoCheckInScheduler>,
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
        
        let db_filename = if cfg!(debug_assertions) {
            "neuradock_dev.db"
        } else {
            "neuradock.db"
        };
        
        let db_path = app_data_dir.join(db_filename);
        let db_path_str = db_path.to_str()
            .ok_or("Invalid database path")?;

        eprintln!("Database path: {}", db_path_str);

        let database = Database::new(db_path_str).await?;
        database.run_migrations().await?;

        let pool = Arc::new(database.pool().clone());
        let db = Arc::new(database);
        let account_repo = Arc::new(SqliteAccountRepository::new(pool.clone())) as Arc<dyn AccountRepository>;

        // Initialize scheduler
        let scheduler = Arc::new(AutoCheckInScheduler::new(
            account_repo.clone(),
        ).await?);
        
        scheduler.start().await?;
        
        // Load existing schedules from database
        eprintln!("Loading auto check-in schedules...");
        use crate::presentation::commands::get_builtin_providers;
        let providers = get_builtin_providers();
        if let Err(e) = scheduler.reload_schedules(
            providers,
            account_repo.clone(),
            app_handle.clone(),
        ).await {
            eprintln!("Failed to load schedules: {}", e);
        } else {
            eprintln!("Auto check-in schedules loaded successfully");
        }

        Ok(Self {
            pool,
            db,
            account_repo,
            scheduler,
            app_handle,
        })
    }
}
