use crate::application::commands::command_handler::CommandHandler;
use crate::application::commands::provider_commands::*;
use crate::application::dtos::{AddProviderInput, BrowserInfoDto, ProviderDto};
use crate::presentation::error::CommandError;
use crate::presentation::state::AppState;
use tauri::State;

/// Add a provider (deprecated - use create_provider instead)
#[tauri::command]
#[specta::specta]
pub async fn add_provider(
    input: AddProviderInput,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    let _ = (input, state);
    Err(CommandError::infrastructure(
        "Not implemented yet - use create_provider instead",
    ))
}

/// Check if a Chromium-based browser is available for WAF bypass
#[tauri::command]
#[specta::specta]
pub async fn check_browser_available() -> Result<BrowserInfoDto, CommandError> {
    use neuradock_infrastructure::http::waf_bypass::check_available_browser;

    match check_available_browser() {
        Some(path) => {
            log::info!("Browser found at: {}", path);
            Ok(BrowserInfoDto {
                available: true,
                path: Some(path),
                message: Some("Browser is available for WAF bypass".to_string()),
            })
        }
        None => {
            log::warn!("No Chromium-based browser found on system");
            Ok(BrowserInfoDto {
                available: false,
                path: None,
                message: Some("No browser found. Please install Chrome, Chromium, Brave, or Microsoft Edge for WAF bypass functionality.".to_string()),
            })
        }
    }
}

/// Get all providers (builtin + custom)
#[tauri::command]
#[specta::specta]
pub async fn get_all_providers(state: State<'_, AppState>) -> Result<Vec<ProviderDto>, CommandError> {
    log::info!("🔍 get_all_providers called");

    // Get all providers (builtin + custom from database)
    let all_providers = state
        .provider_repo
        .find_all()
        .await
        .map_err(CommandError::from)?;

    log::info!("📊 Found {} providers from database", all_providers.len());
    for p in &all_providers {
        log::info!("  - DB provider: {} ({})", p.name(), p.id().as_str());
    }

    log::info!("📊 Total providers loaded: {}", all_providers.len());

    let accounts = state
        .account_repo
        .find_all()
        .await
        .map_err(CommandError::from)?;

    let dtos: Vec<ProviderDto> = all_providers
        .iter()
        .map(|provider| {
            let account_count = accounts
                .iter()
                .filter(|acc| acc.provider_id() == provider.id())
                .count();

            ProviderDto {
                id: provider.id().as_str().to_string(),
                name: provider.name().to_string(),
                domain: provider.domain().to_string(),
                is_builtin: provider.is_builtin(),
                account_count: account_count as i32,
                supports_check_in: provider.supports_check_in(),
                check_in_bugged: provider.check_in_bugged(),
                // API configuration
                login_path: provider
                    .login_url()
                    .trim_start_matches(provider.domain())
                    .to_string(),
                sign_in_path: provider
                    .sign_in_url()
                    .as_ref()
                    .map(|url| url.trim_start_matches(provider.domain()).to_string()),
                user_info_path: provider
                    .user_info_url()
                    .trim_start_matches(provider.domain())
                    .to_string(),
                token_api_path: provider
                    .token_api_url()
                    .as_ref()
                    .map(|url| url.trim_start_matches(provider.domain()).to_string()),
                models_path: provider
                    .models_url()
                    .as_ref()
                    .map(|url| url.trim_start_matches(provider.domain()).to_string()),
                api_user_key: provider.api_user_key().to_string(),
                needs_waf_bypass: provider.needs_waf_bypass(),
            }
        })
        .collect();

    log::info!("✅ Returning {} provider DTOs", dtos.len());
    for dto in &dtos {
        log::info!(
            "  - DTO: {} ({}) builtin={}",
            dto.name,
            dto.id,
            dto.is_builtin
        );
    }

    Ok(dtos)
}

/// Create a custom provider
#[tauri::command]
#[specta::specta]
pub async fn create_provider(
    input: CreateProviderCommand,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    let result = state
        .command_handlers
        .create_provider
        .handle(input)
        .await
        .map_err(CommandError::from)?;

    Ok(result.provider_id)
}

/// Update a custom provider
#[tauri::command]
#[specta::specta]
pub async fn update_provider(
    input: UpdateProviderCommand,
    state: State<'_, AppState>,
) -> Result<bool, CommandError> {
    state
        .command_handlers
        .update_provider
        .handle(input)
        .await
        .map_err(CommandError::from)?;

    Ok(true)
}

/// Delete a custom provider
#[tauri::command]
#[specta::specta]
pub async fn delete_provider(
    input: DeleteProviderCommand,
    state: State<'_, AppState>,
) -> Result<bool, CommandError> {
    state
        .command_handlers
        .delete_provider
        .handle(input)
        .await
        .map_err(CommandError::from)?;

    Ok(true)
}
