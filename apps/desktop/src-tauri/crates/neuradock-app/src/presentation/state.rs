use std::sync::Arc;

use crate::application::commands::handlers::*;
use crate::application::queries::{
    AccountQueryService, BalanceStatisticsQueryService, CheckInStreakQueries,
};
use crate::application::services::{
    BalanceService, ClaudeConfigService, CodexConfigService, ConfigService,
    ProviderModelsQueryService, ProxyConfigService, TokenService,
};
use neuradock_domain::account::AccountRepository;
use neuradock_domain::ai_chat::AiChatServiceRepository;
use neuradock_domain::check_in::ProviderRepository;
use neuradock_domain::custom_node::CustomProviderNodeRepository;
use neuradock_domain::independent_key::IndependentKeyRepository;
use neuradock_domain::notification::NotificationChannelRepository;
use neuradock_domain::session::SessionRepository;

/// Command handlers container
#[derive(Clone)]
pub struct CommandHandlers {
    pub create_account: Arc<CreateAccountCommandHandler>,
    pub update_account: Arc<UpdateAccountCommandHandler>,
    pub delete_account: Arc<DeleteAccountCommandHandler>,
    pub toggle_account: Arc<ToggleAccountCommandHandler>,
    pub execute_check_in: Arc<ExecuteCheckInCommandHandler>,
    pub batch_execute_check_in: Arc<BatchExecuteCheckInCommandHandler>,
    pub create_notification_channel: Arc<CreateNotificationChannelHandler>,
    pub update_notification_channel: Arc<UpdateNotificationChannelHandler>,
    pub delete_notification_channel: Arc<DeleteNotificationChannelHandler>,
    pub test_notification_channel: Arc<TestNotificationChannelHandler>,
    pub create_provider: Arc<CreateProviderCommandHandler>,
    pub update_provider: Arc<UpdateProviderCommandHandler>,
    pub delete_provider: Arc<DeleteProviderCommandHandler>,
}

#[derive(Clone)]
pub struct Repositories {
    pub account: Arc<dyn AccountRepository>,
    pub session: Arc<dyn SessionRepository>,
    pub notification_channel: Arc<dyn NotificationChannelRepository>,
    pub custom_node: Arc<dyn CustomProviderNodeRepository>,
    pub independent_key: Arc<dyn IndependentKeyRepository>,
    pub provider: Arc<dyn ProviderRepository>,
    pub ai_chat_service: Arc<dyn AiChatServiceRepository>,
}

#[derive(Clone)]
pub struct Services {
    pub token: Arc<TokenService>,
    pub claude_config: Arc<ClaudeConfigService>,
    pub codex_config: Arc<CodexConfigService>,
    pub config: Arc<ConfigService>,
    pub balance: Arc<BalanceService>,
    pub proxy_config: Arc<ProxyConfigService>,
    pub provider_models_query: Arc<ProviderModelsQueryService>,
}

#[derive(Clone)]
pub struct Queries {
    pub account: Arc<AccountQueryService>,
    pub streak: Arc<CheckInStreakQueries>,
    pub balance_statistics: Arc<BalanceStatisticsQueryService>,
}

#[derive(Clone)]
pub struct AppState {
    pub repositories: Repositories,
    pub services: Services,
    pub queries: Queries,
    pub command_handlers: CommandHandlers,
}

impl AppState {
    pub async fn new(app_handle: tauri::AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        crate::presentation::bootstrap::build_app_state(app_handle).await
    }
}
