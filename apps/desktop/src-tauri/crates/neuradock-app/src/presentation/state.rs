use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Manager;
use tracing::{info, warn, error};

use crate::application::commands::handlers::*;
use crate::application::event_handlers::SchedulerReloadEventHandler;
use crate::application::queries::{AccountQueryService, CheckInStreakQueries};
use crate::application::services::{AutoCheckInScheduler, ConfigService};
use neuradock_domain::events::account_events::*;
use neuradock_domain::events::EventBus;
use neuradock_domain::account::AccountRepository;
use neuradock_domain::check_in::Provider;
use neuradock_infrastructure::events::InMemoryEventBus;
use neuradock_infrastructure::persistence::{repositories::SqliteAccountRepository, Database};
use neuradock_infrastructure::security::{EncryptionService, KeyManager};

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
    pub encryption_service: Arc<EncryptionService>,
    pub config_service: Arc<ConfigService>,
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

        info!("Database path: {}", db_path_str);

        // Initialize encryption
        info!("🔐 Initializing encryption...");
        let key_manager = KeyManager::new(app_data_dir.clone());
        let salt = key_manager.initialize()
            .map_err(|e| format!("Failed to initialize encryption salt: {}", e))?;
        
        // TODO: In production, get password from secure input
        // For now, use a default password (should be configurable)
        let encryption_password = "neuradock_default_password_2024";
        let encryption_service = Arc::new(
            EncryptionService::from_password(encryption_password, &salt)
                .map_err(|e| format!("Failed to create encryption service: {}", e))?
        );
        info!("✓ Encryption initialized");

        info!("🔌 Connecting to database...");
        let database = Database::new(db_path_str).await?;
        info!("✓ Database connection established");

        info!("🔄 Running migrations...");
        database.run_migrations().await?;
        info!("✓ Migrations completed");

        let pool = Arc::new(database.pool().clone());
        let db = Arc::new(database);
        let account_repo =
            Arc::new(SqliteAccountRepository::new(pool.clone(), encryption_service.clone())) as Arc<dyn AccountRepository>;
        let account_queries = Arc::new(AccountQueryService::new(account_repo.clone()));
        let streak_queries = Arc::new(CheckInStreakQueries::new(pool.clone()));

        info!("📊 Initializing scheduler...");
        // Initialize scheduler
        let scheduler = Arc::new(AutoCheckInScheduler::new(account_repo.clone()).await?);
        info!("✓ Scheduler initialized");

        info!("▶️  Starting scheduler...");
        scheduler.start().await?;
        info!("✓ Scheduler started");

        // Initialize event bus and register event handlers
        info!("🔧 Initializing event bus...");
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
        
        use neuradock_domain::events::TypedEventHandlerWrapper;
        
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
        
        info!("✓ Event bus initialized and handlers registered");

        // Load existing schedules from database
        info!("📋 Loading auto check-in schedules...");
        let providers = get_builtin_providers();
        info!("📦 Got {} providers", providers.len());

        // Create a HashMap for command handlers (providers don't need Arc)
        let providers_map: HashMap<String, Provider> = providers
            .iter()
            .map(|(k, v)| (k.to_string(), (*v).clone()))
            .collect();

        if let Err(e) = scheduler
            .reload_schedules(providers.clone(), account_repo.clone(), app_handle.clone())
            .await
        {
            warn!("⚠️  Failed to load schedules: {}", e);
        } else {
            info!("✓ Auto check-in schedules loaded successfully");
        }

        // Initialize command handlers
        info!("🔧 Initializing command handlers...");
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
                providers_map.clone(),
                true, // headless_browser
            )),
            batch_execute_check_in: Arc::new(BatchExecuteCheckInCommandHandler::new(
                account_repo.clone(),
                providers_map,
                true, // headless_browser
            )),
        };
        info!("✓ Command handlers initialized");

        // Initialize config service
        let config_service = Arc::new(ConfigService::new());
        info!("✓ Config service initialized");

        Ok(Self {
            pool,
            db,
            account_repo,
            scheduler,
            event_bus,
            account_queries,
            streak_queries,
            command_handlers,
            encryption_service,
            config_service,
            app_handle,
        })
    }
}
