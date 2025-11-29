use sqlx::SqlitePool;
use std::sync::Arc;
use tauri::Manager;

use crate::application::commands::handlers::*;
use crate::application::event_handlers::SchedulerReloadEventHandler;
use crate::application::queries::{AccountQueryService, CheckInStreakQueries};
use crate::application::services::AutoCheckInScheduler;
use crate::domain::events::account_events::*;
use crate::domain::events::EventBus;
use crate::domain::account::AccountRepository;
use crate::infrastructure::events::InMemoryEventBus;
use crate::infrastructure::persistence::{repositories::SqliteAccountRepository, Database};

/// Command handlers container
pub struct CommandHandlers {
    pub create_account: Arc<CreateAccountCommandHandler>,
    pub update_account: Arc<UpdateAccountCommandHandler>,
    pub delete_account: Arc<DeleteAccountCommandHandler>,
    pub toggle_account: Arc<ToggleAccountCommandHandler>,
    pub execute_check_in: Arc<ExecuteCheckInCommandHandler>,
    pub batch_execute_check_in: Arc<BatchExecuteCheckInCommandHandler>,
}

pub struct AppState {
    pub pool: Arc<SqlitePool>,
    pub db: Arc<Database>,
    pub account_repo: Arc<dyn AccountRepository>,
    pub scheduler: Arc<AutoCheckInScheduler>,
    pub event_bus: Arc<dyn EventBus>,
    pub account_queries: Arc<AccountQueryService>,
    pub streak_queries: Arc<CheckInStreakQueries>,
    pub command_handlers: CommandHandlers,
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
        let account_queries = Arc::new(AccountQueryService::new(account_repo.clone()));
        let streak_queries = Arc::new(CheckInStreakQueries::new(pool.clone()));

        eprintln!("📊 Initializing scheduler...");
        // Initialize scheduler
        let scheduler = Arc::new(AutoCheckInScheduler::new(account_repo.clone()).await?);
        eprintln!("✓ Scheduler initialized");

        eprintln!("▶️  Starting scheduler...");
        scheduler.start().await?;
        eprintln!("✓ Scheduler started");

        // Initialize event bus and register event handlers
        eprintln!("🔧 Initializing event bus...");
        let event_bus = Arc::new(InMemoryEventBus::new());
        
        // Get providers for event handlers
        use crate::presentation::commands::get_builtin_providers;
        let providers = get_builtin_providers();
        
        // Register SchedulerReloadEventHandler for account events
        let scheduler_reload_handler = SchedulerReloadEventHandler::new(
            scheduler.clone(),
            account_repo.clone(),
            providers.clone(),
            app_handle.clone(),
        );
        
        use crate::domain::events::TypedEventHandlerWrapper;
        
        event_bus.subscribe::<AccountCreated>(
            Arc::new(TypedEventHandlerWrapper::<AccountCreated, _>::new(scheduler_reload_handler.clone()))
        ).await;
        event_bus.subscribe::<AccountUpdated>(
            Arc::new(TypedEventHandlerWrapper::<AccountUpdated, _>::new(scheduler_reload_handler.clone()))
        ).await;
        event_bus.subscribe::<AccountDeleted>(
            Arc::new(TypedEventHandlerWrapper::<AccountDeleted, _>::new(scheduler_reload_handler.clone()))
        ).await;
        event_bus.subscribe::<AccountToggled>(
            Arc::new(TypedEventHandlerWrapper::<AccountToggled, _>::new(scheduler_reload_handler))
        ).await;
        
        eprintln!("✓ Event bus initialized and handlers registered");

        // Load existing schedules from database
        eprintln!("📋 Loading auto check-in schedules...");
        eprintln!("📦 Got {} providers", providers.len());
        let providers = get_builtin_providers();
        eprintln!("📦 Got {} providers", providers.len());

        eprintln!("🔍 Calling reload_schedules...");
        if let Err(e) = scheduler
            .reload_schedules(providers.clone(), account_repo.clone(), app_handle.clone())
            .await
        {
            eprintln!("⚠️  Failed to load schedules: {}", e);
        } else {
            eprintln!("✓ Auto check-in schedules loaded successfully");
        }

        // Initialize command handlers
        eprintln!("🔧 Initializing command handlers...");
        let command_handlers = CommandHandlers {
            create_account: Arc::new(CreateAccountCommandHandler::new(
                account_repo.clone(),
                event_bus.clone(),
            )),
            update_account: Arc::new(UpdateAccountCommandHandler::new(
                account_repo.clone(),
                event_bus.clone(),
            )),
            delete_account: Arc::new(DeleteAccountCommandHandler::new(
                account_repo.clone(),
                event_bus.clone(),
            )),
            toggle_account: Arc::new(ToggleAccountCommandHandler::new(
                account_repo.clone(),
                event_bus.clone(),
            )),
            execute_check_in: Arc::new(ExecuteCheckInCommandHandler::new(
                account_repo.clone(),
                true, // headless_browser
            )),
            batch_execute_check_in: Arc::new(BatchExecuteCheckInCommandHandler::new(
                account_repo.clone(),
                true, // headless_browser
            )),
        };
        eprintln!("✓ Command handlers initialized");

        Ok(Self {
            pool,
            db,
            account_repo,
            scheduler,
            event_bus,
            account_queries,
            streak_queries,
            command_handlers,
            app_handle,
        })
    }
}
