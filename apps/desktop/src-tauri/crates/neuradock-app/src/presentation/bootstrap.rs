use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tauri::Manager;
use tracing::{info, warn};

use crate::application::commands::handlers::*;
use crate::application::event_handlers::SchedulerReloadEventHandler;
use crate::application::queries::BalanceStatisticsQueryService;
use crate::application::queries::{AccountQueryService, CheckInStreakQueries};
use crate::application::services::{
    AutoCheckInScheduler, BalanceHistoryService, BalanceService, ClaudeConfigService,
    CodexConfigService, ConfigService, NotificationService, ProviderModelsQueryService,
    ProviderModelsService, ProxyConfigService, TokenService,
};
use crate::presentation::state::{AppState, CommandHandlers, Queries, Repositories, Services};
use neuradock_domain::account::AccountRepository;
use neuradock_domain::ai_chat::AiChatServiceRepository;
use neuradock_domain::balance_history::BalanceHistoryRepository;
use neuradock_domain::check_in::{Provider, ProviderRepository};
use neuradock_domain::custom_node::CustomProviderNodeRepository;
use neuradock_domain::events::account_events::*;
use neuradock_domain::independent_key::IndependentKeyRepository;
use neuradock_domain::notification::NotificationChannelRepository;
use neuradock_domain::provider_models::ProviderModelsRepository;
use neuradock_domain::proxy_config::ProxyConfigRepository;
use neuradock_domain::session::SessionRepository;
use neuradock_domain::token::TokenRepository;
use neuradock_domain::waf_cookies::WafCookiesRepository;
use neuradock_infrastructure::bootstrap::seed_builtin_providers;
use neuradock_infrastructure::bootstrap::seed_builtin_ai_chats;
use neuradock_infrastructure::events::InMemoryEventBus;
use neuradock_infrastructure::notification::SqliteNotificationChannelRepository;
use neuradock_infrastructure::persistence::{
    repositories::{
        SqliteAccountRepository, SqliteAiChatServiceRepository, SqliteBalanceHistoryRepository,
        SqliteCustomProviderNodeRepository, SqliteIndependentKeyRepository,
        SqliteProviderModelsRepository, SqliteProviderRepository, SqliteProxyConfigRepository,
        SqliteSessionRepository, SqliteTokenRepository, SqliteWafCookiesRepository,
    },
    Database,
};
use neuradock_infrastructure::security::{EncryptionService, KeyManager};

pub async fn build_app_state(
    app_handle: tauri::AppHandle,
) -> Result<AppState, Box<dyn std::error::Error>> {
    let startup_started_at = Instant::now();

    // Get app data directory (~/Library/Application Support/com.neuradock.app/)
    let started_at = Instant::now();
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    info!(
        "✓ Resolved app data dir ({}ms)",
        started_at.elapsed().as_millis()
    );

    // Create directory if it doesn't exist
    let started_at = Instant::now();
    std::fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data directory: {}", e))?;
    info!(
        "✓ Ensured app data dir exists ({}ms)",
        started_at.elapsed().as_millis()
    );

    let db_filename = if cfg!(debug_assertions) {
        "neuradock-dev.db"
    } else {
        "neuradock.db"
    };

    let db_path = app_data_dir.join(db_filename);
    let db_path_str = db_path.to_str().ok_or("Invalid database path")?;

    info!("Database path: {}", db_path_str);

    // Initialize encryption
    info!("🔐 Initializing encryption...");
    let started_at = Instant::now();
    let key_manager = KeyManager::new(app_data_dir.clone());
    let salt = key_manager
        .initialize()
        .map_err(|e| format!("Failed to initialize encryption salt: {}", e))?;

    // TODO: In production, get password from secure input
    // For now, use a default password (should be configurable)
    let encryption_password = "neuradock_default_password_2024";
    let encryption_service = Arc::new(
        EncryptionService::from_password(encryption_password, &salt)
            .map_err(|e| format!("Failed to create encryption service: {}", e))?,
    );
    info!(
        "✓ Encryption initialized ({}ms)",
        started_at.elapsed().as_millis()
    );

    info!("🔌 Connecting to database...");
    let started_at = Instant::now();
    let database = Database::new(db_path_str).await?;
    info!(
        "✓ Database connection established ({}ms)",
        started_at.elapsed().as_millis()
    );

    info!("🔄 Running migrations...");
    let started_at = Instant::now();
    database.run_migrations().await?;
    info!(
        "✓ Migrations completed ({}ms)",
        started_at.elapsed().as_millis()
    );

    let pool = Arc::new(database.pool().clone());

    let account_repo = Arc::new(SqliteAccountRepository::new(
        pool.clone(),
        encryption_service.clone(),
    )) as Arc<dyn AccountRepository>;
    let session_repo =
        Arc::new(SqliteSessionRepository::new(pool.clone())) as Arc<dyn SessionRepository>;
    let notification_channel_repo = Arc::new(SqliteNotificationChannelRepository::new(pool.clone()))
        as Arc<dyn NotificationChannelRepository>;
    let token_repo = Arc::new(SqliteTokenRepository::new(pool.clone())) as Arc<dyn TokenRepository>;
    let custom_node_repo = Arc::new(SqliteCustomProviderNodeRepository::new(pool.clone()))
        as Arc<dyn CustomProviderNodeRepository>;
    let independent_key_repo = Arc::new(SqliteIndependentKeyRepository::new(
        pool.clone(),
        encryption_service.clone(),
    )) as Arc<dyn IndependentKeyRepository>;
    let provider_repo =
        Arc::new(SqliteProviderRepository::new(pool.clone())) as Arc<dyn ProviderRepository>;
    let provider_models_repo = Arc::new(SqliteProviderModelsRepository::new(pool.clone()))
        as Arc<dyn ProviderModelsRepository>;
    let waf_cookies_repo =
        Arc::new(SqliteWafCookiesRepository::new(pool.clone())) as Arc<dyn WafCookiesRepository>;
    let proxy_config_repo =
        Arc::new(SqliteProxyConfigRepository::new(pool.clone())) as Arc<dyn ProxyConfigRepository>;
    let balance_history_repo = Arc::new(SqliteBalanceHistoryRepository::new(pool.clone()))
        as Arc<dyn BalanceHistoryRepository>;
    let ai_chat_service_repo = Arc::new(SqliteAiChatServiceRepository::new(pool.clone()))
        as Arc<dyn AiChatServiceRepository>;

    info!("🌱 Seeding built-in providers...");
    let started_at = Instant::now();
    seed_builtin_providers(
        provider_repo.clone(),
        custom_node_repo.clone(),
        account_repo.clone(),
        provider_models_repo.clone(),
        waf_cookies_repo.clone(),
    )
        .await
        .map_err(|e| format!("Failed to seed built-in providers: {}", e))?;
    info!(
        "✓ Built-in providers seeded ({}ms)",
        started_at.elapsed().as_millis()
    );

    info!("🌱 Seeding built-in AI chat services...");
    let started_at = Instant::now();
    seed_builtin_ai_chats(ai_chat_service_repo.clone())
        .await
        .map_err(|e| format!("Failed to seed built-in AI chat services: {}", e))?;
    info!(
        "✓ Built-in AI chat services seeded ({}ms)",
        started_at.elapsed().as_millis()
    );

    let notification_service = Arc::new(NotificationService::new(
        notification_channel_repo.clone(),
        balance_history_repo.clone(),
    ));
    let token_service = build_token_service(
        token_repo.clone(),
        account_repo.clone(),
        provider_repo.clone(),
        proxy_config_repo.clone(),
        waf_cookies_repo.clone(),
    )?;
    let claude_config_service = Arc::new(ClaudeConfigService::new());
    let codex_config_service = Arc::new(CodexConfigService::new());
    let config_service = build_config_service(&app_handle)?;

    let account_queries = Arc::new(AccountQueryService::new(account_repo.clone()));
    let streak_queries = Arc::new(CheckInStreakQueries::new(
        account_repo.clone(),
        provider_repo.clone(),
        balance_history_repo.clone(),
    ));

    // Initialize check-in related services
    let provider_models_service = Arc::new(ProviderModelsService::new(
        provider_models_repo.clone(),
        waf_cookies_repo.clone(),
        proxy_config_repo.clone(),
    ));
    let provider_models_query = Arc::new(ProviderModelsQueryService::new(
        account_repo.clone(),
        provider_repo.clone(),
        provider_models_repo.clone(),
        waf_cookies_repo.clone(),
        proxy_config_repo.clone(),
    ));
    let balance_history_service = Arc::new(BalanceHistoryService::new(balance_history_repo));
    let balance_service = Arc::new(BalanceService::new(
        account_repo.clone(),
        provider_repo.clone(),
        balance_history_service.clone(),
        proxy_config_repo.clone(),
        true,
    ));
    let balance_statistics_queries = Arc::new(BalanceStatisticsQueryService::new(
        account_repo.clone(),
        provider_repo.clone(),
        balance_history_service.clone(),
    ));

    info!("📊 Initializing scheduler...");
    let started_at = Instant::now();
    let scheduler = Arc::new(AutoCheckInScheduler::new().await?);
    info!(
        "✓ Scheduler initialized ({}ms)",
        started_at.elapsed().as_millis()
    );

    info!("▶️  Starting scheduler...");
    let started_at = Instant::now();
    scheduler.start().await?;
    info!(
        "✓ Scheduler started ({}ms)",
        started_at.elapsed().as_millis()
    );

    // Initialize event bus and register event handlers
    info!("🔧 Initializing event bus...");
    let event_bus = Arc::new(InMemoryEventBus::new());

    // Register SchedulerReloadEventHandler for account events
    let scheduler_reload_handler = SchedulerReloadEventHandler::new(
        scheduler.clone(),
        account_repo.clone(),
        provider_repo.clone(),
        app_handle.clone(),
    );

    use neuradock_domain::events::TypedEventHandlerWrapper;

    let _ = event_bus
        .subscribe::<AccountCreated>(Arc::new(
            TypedEventHandlerWrapper::<AccountCreated, _>::new(scheduler_reload_handler.clone()),
        ))
        .await;
    let _ = event_bus
        .subscribe::<AccountUpdated>(Arc::new(
            TypedEventHandlerWrapper::<AccountUpdated, _>::new(scheduler_reload_handler.clone()),
        ))
        .await;
    let _ = event_bus
        .subscribe::<AccountDeleted>(Arc::new(
            TypedEventHandlerWrapper::<AccountDeleted, _>::new(scheduler_reload_handler.clone()),
        ))
        .await;
    let _ = event_bus
        .subscribe::<AccountToggled>(Arc::new(
            TypedEventHandlerWrapper::<AccountToggled, _>::new(scheduler_reload_handler),
        ))
        .await;

    info!("✓ Event bus initialized and handlers registered");

    // Load existing schedules from database
    info!("📋 Loading auto check-in schedules...");
    let started_at = Instant::now();
    let provider_list = provider_repo
        .find_all()
        .await
        .map_err(|e| format!("Failed to load providers: {}", e))?;
    info!("📦 Got {} providers", provider_list.len());

    let providers_map: HashMap<String, Provider> = provider_list
        .into_iter()
        .map(|provider| (provider.id().as_str().to_string(), provider))
        .collect();

    if let Err(e) = scheduler
        .reload_schedules(providers_map, account_repo.clone(), app_handle.clone())
        .await
    {
        warn!("⚠️  Failed to load schedules: {}", e);
    } else {
        info!(
            "✓ Auto check-in schedules loaded ({}ms)",
            started_at.elapsed().as_millis()
        );
    }

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
        execute_check_in: Arc::new(
            ExecuteCheckInCommandHandler::new(
                account_repo.clone(),
                provider_repo.clone(),
                proxy_config_repo.clone(),
                provider_models_service.clone(),
                balance_history_service.clone(),
                waf_cookies_repo.clone(),
                true, // headless_browser
            )
            .with_notification_service(notification_service.clone()),
        ),
        batch_execute_check_in: Arc::new(
            BatchExecuteCheckInCommandHandler::new(
                account_repo.clone(),
                provider_repo.clone(),
                proxy_config_repo.clone(),
                provider_models_service.clone(),
                balance_history_service.clone(),
                waf_cookies_repo.clone(),
                true, // headless_browser
            )
            .with_notification_service(notification_service.clone()),
        ),
        create_notification_channel: Arc::new(CreateNotificationChannelHandler::new(
            notification_channel_repo.clone(),
        )),
        update_notification_channel: Arc::new(UpdateNotificationChannelHandler::new(
            notification_channel_repo.clone(),
        )),
        delete_notification_channel: Arc::new(DeleteNotificationChannelHandler::new(
            notification_channel_repo.clone(),
        )),
        test_notification_channel: Arc::new(TestNotificationChannelHandler::new(
            notification_channel_repo.clone(),
        )),
        create_provider: Arc::new(CreateProviderCommandHandler::new(provider_repo.clone())),
        update_provider: Arc::new(UpdateProviderCommandHandler::new(provider_repo.clone())),
        delete_provider: Arc::new(DeleteProviderCommandHandler::new(provider_repo.clone())),
    };
    info!("✓ Command handlers initialized");

    info!(
        "✅ AppState ready ({}ms)",
        startup_started_at.elapsed().as_millis()
    );

    Ok(AppState {
        repositories: Repositories {
            account: account_repo,
            session: session_repo,
            notification_channel: notification_channel_repo,
            custom_node: custom_node_repo,
            independent_key: independent_key_repo,
            provider: provider_repo,
            ai_chat_service: ai_chat_service_repo,
        },
        services: Services {
            token: token_service,
            claude_config: claude_config_service,
            codex_config: codex_config_service,
            config: config_service,
            balance: balance_service,
            proxy_config: Arc::new(ProxyConfigService::new(proxy_config_repo.clone())),
            provider_models_query,
        },
        queries: Queries {
            account: account_queries,
            streak: streak_queries,
            balance_statistics: balance_statistics_queries,
        },
        command_handlers,
    })
}

fn build_config_service(
    app_handle: &tauri::AppHandle,
) -> Result<Arc<ConfigService>, Box<dyn std::error::Error>> {
    info!("🔧 Initializing config service...");
    let started_at = Instant::now();
    let service = Arc::new(
        ConfigService::new(app_handle)
            .map_err(|e| format!("Failed to initialize config service: {}", e))?,
    );
    info!(
        "✓ Config service initialized ({}ms)",
        started_at.elapsed().as_millis()
    );
    Ok(service)
}

fn build_token_service(
    token_repo: Arc<dyn TokenRepository>,
    account_repo: Arc<dyn AccountRepository>,
    provider_repo: Arc<dyn ProviderRepository>,
    proxy_config_repo: Arc<dyn ProxyConfigRepository>,
    waf_cookies_repo: Arc<dyn WafCookiesRepository>,
) -> Result<Arc<TokenService>, Box<dyn std::error::Error>> {
    info!("🔧 Initializing token services...");
    let started_at = Instant::now();
    let service = Arc::new(
        TokenService::new(token_repo, account_repo, provider_repo, proxy_config_repo)
            .map_err(|e| format!("Failed to initialize token service: {}", e))?
            .with_waf_cookies_repo(waf_cookies_repo),
    );
    info!(
        "✓ Token services initialized ({}ms)",
        started_at.elapsed().as_millis()
    );
    Ok(service)
}
