use crate::application::dtos::{
    AiChatServiceDto, CreateAiChatServiceInput, ReorderAiChatServicesInput,
    UpdateAiChatServiceInput,
};
use crate::presentation::error::CommandError;
use crate::presentation::state::Repositories;
use neuradock_domain::ai_chat::{AiChatService, AiChatServiceId};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

/// List all AI chat services (both enabled and disabled)
#[tauri::command]
#[specta::specta]
pub async fn list_ai_chat_services(
    repositories: State<'_, Repositories>,
) -> Result<Vec<AiChatServiceDto>, CommandError> {
    log::info!("🔍 list_ai_chat_services called");

    let services = repositories
        .ai_chat_service
        .find_all()
        .await
        .map_err(CommandError::from)?;

    log::info!("📊 Found {} AI chat services", services.len());

    let dtos: Vec<AiChatServiceDto> = services.into_iter().map(AiChatServiceDto::from).collect();

    Ok(dtos)
}

/// List only enabled AI chat services
#[tauri::command]
#[specta::specta]
pub async fn list_enabled_ai_chat_services(
    repositories: State<'_, Repositories>,
) -> Result<Vec<AiChatServiceDto>, CommandError> {
    log::info!("🔍 list_enabled_ai_chat_services called");

    let services = repositories
        .ai_chat_service
        .find_enabled()
        .await
        .map_err(CommandError::from)?;

    log::info!("📊 Found {} enabled AI chat services", services.len());

    let dtos: Vec<AiChatServiceDto> = services.into_iter().map(AiChatServiceDto::from).collect();

    Ok(dtos)
}

/// Create a custom AI chat service
#[tauri::command]
#[specta::specta]
pub async fn create_ai_chat_service(
    input: CreateAiChatServiceInput,
    repositories: State<'_, Repositories>,
) -> Result<AiChatServiceDto, CommandError> {
    log::info!("🆕 create_ai_chat_service called: {}", input.name);

    let service = AiChatService::new(input.name, input.url, input.icon)
        .map_err(CommandError::from)?;

    repositories
        .ai_chat_service
        .save(&service)
        .await
        .map_err(CommandError::from)?;

    log::info!("✅ AI chat service created: {}", service.id());

    Ok(AiChatServiceDto::from(&service))
}

/// Update an AI chat service (only custom services can be fully updated)
#[tauri::command]
#[specta::specta]
pub async fn update_ai_chat_service(
    input: UpdateAiChatServiceInput,
    repositories: State<'_, Repositories>,
) -> Result<AiChatServiceDto, CommandError> {
    log::info!("✏️  update_ai_chat_service called: {}", input.id);

    let id = AiChatServiceId::from_string(&input.id);
    let mut service = repositories
        .ai_chat_service
        .find_by_id(&id)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found(format!("AI chat service not found: {}", input.id)))?;

    service
        .update(input.name, input.url, input.icon)
        .map_err(CommandError::from)?;

    repositories
        .ai_chat_service
        .save(&service)
        .await
        .map_err(CommandError::from)?;

    log::info!("✅ AI chat service updated: {}", service.id());

    Ok(AiChatServiceDto::from(&service))
}

/// Delete a custom AI chat service (built-in services cannot be deleted)
#[tauri::command]
#[specta::specta]
pub async fn delete_ai_chat_service(
    id: String,
    repositories: State<'_, Repositories>,
) -> Result<bool, CommandError> {
    log::info!("🗑️  delete_ai_chat_service called: {}", id);

    let service_id = AiChatServiceId::from_string(&id);

    repositories
        .ai_chat_service
        .delete(&service_id)
        .await
        .map_err(CommandError::from)?;

    log::info!("✅ AI chat service deleted: {}", id);

    Ok(true)
}

/// Toggle an AI chat service's enabled state
#[tauri::command]
#[specta::specta]
pub async fn toggle_ai_chat_service(
    id: String,
    repositories: State<'_, Repositories>,
) -> Result<AiChatServiceDto, CommandError> {
    log::info!("🔄 toggle_ai_chat_service called: {}", id);

    let service_id = AiChatServiceId::from_string(&id);
    let mut service = repositories
        .ai_chat_service
        .find_by_id(&service_id)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found(format!("AI chat service not found: {}", id)))?;

    service.toggle();

    repositories
        .ai_chat_service
        .save(&service)
        .await
        .map_err(CommandError::from)?;

    log::info!(
        "✅ AI chat service toggled: {} (enabled={})",
        id,
        service.is_enabled()
    );

    Ok(AiChatServiceDto::from(&service))
}

/// Reorder AI chat services
#[tauri::command]
#[specta::specta]
pub async fn reorder_ai_chat_services(
    input: ReorderAiChatServicesInput,
    repositories: State<'_, Repositories>,
) -> Result<bool, CommandError> {
    log::info!(
        "↕️  reorder_ai_chat_services called: {} services",
        input.service_ids.len()
    );

    let orders: Vec<(AiChatServiceId, i32)> = input
        .service_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (AiChatServiceId::from_string(id), index as i32))
        .collect();

    repositories
        .ai_chat_service
        .update_sort_orders(&orders)
        .await
        .map_err(CommandError::from)?;

    log::info!("✅ AI chat services reordered");

    Ok(true)
}

// ============================================================================
// WebView Management Commands
// ============================================================================

/// Open an AI chat service in its own WebView window.
/// If the window already exists, it will be focused instead.
#[tauri::command]
#[specta::specta]
pub async fn open_ai_chat_webview(
    app: AppHandle,
    service_id: String,
    service_name: String,
    url: String,
) -> Result<bool, CommandError> {
    log::info!("🌐 open_ai_chat_webview called: {} ({})", service_name, url);

    let window_label = format!("ai-chat-{}", service_id);

    // Check if window already exists
    if let Some(window) = app.get_webview_window(&window_label) {
        log::info!("🔄 Window already exists, focusing: {}", window_label);
        window.set_focus().map_err(|e| {
            CommandError::infrastructure(format!("Failed to focus window: {}", e))
        })?;
        window.unminimize().map_err(|e| {
            CommandError::infrastructure(format!("Failed to unminimize window: {}", e))
        })?;
        return Ok(true);
    }

    // Parse URL
    let parsed_url: url::Url = url.parse().map_err(|e| {
        CommandError::validation(format!("Invalid URL: {}", e))
    })?;

    // Create new WebView window
    log::info!("🆕 Creating new WebView window: {}", window_label);
    let window = WebviewWindowBuilder::new(&app, &window_label, WebviewUrl::External(parsed_url))
        .title(&service_name)
        .inner_size(1200.0, 800.0)
        .min_inner_size(800.0, 600.0)
        .center()
        .visible(true)
        .build()
        .map_err(|e| {
            CommandError::infrastructure(format!("Failed to create WebView window: {}", e))
        })?;

    log::info!("✅ WebView window created: {}", window_label);

    // Focus the new window
    window.set_focus().map_err(|e| {
        CommandError::infrastructure(format!("Failed to focus new window: {}", e))
    })?;

    Ok(true)
}

/// Close an AI chat WebView window
#[tauri::command]
#[specta::specta]
pub async fn close_ai_chat_webview(
    app: AppHandle,
    service_id: String,
) -> Result<bool, CommandError> {
    log::info!("🚪 close_ai_chat_webview called: {}", service_id);

    let window_label = format!("ai-chat-{}", service_id);

    if let Some(window) = app.get_webview_window(&window_label) {
        window.close().map_err(|e| {
            CommandError::infrastructure(format!("Failed to close window: {}", e))
        })?;
        log::info!("✅ WebView window closed: {}", window_label);
        Ok(true)
    } else {
        log::warn!("⚠️  Window not found: {}", window_label);
        Ok(false)
    }
}

/// Focus an AI chat WebView window
#[tauri::command]
#[specta::specta]
pub async fn focus_ai_chat_webview(
    app: AppHandle,
    service_id: String,
) -> Result<bool, CommandError> {
    log::info!("🎯 focus_ai_chat_webview called: {}", service_id);

    let window_label = format!("ai-chat-{}", service_id);

    if let Some(window) = app.get_webview_window(&window_label) {
        window.set_focus().map_err(|e| {
            CommandError::infrastructure(format!("Failed to focus window: {}", e))
        })?;
        window.unminimize().map_err(|e| {
            CommandError::infrastructure(format!("Failed to unminimize window: {}", e))
        })?;
        log::info!("✅ WebView window focused: {}", window_label);
        Ok(true)
    } else {
        log::warn!("⚠️  Window not found: {}", window_label);
        Ok(false)
    }
}

/// Check if an AI chat WebView window is open
#[tauri::command]
#[specta::specta]
pub async fn is_ai_chat_webview_open(
    app: AppHandle,
    service_id: String,
) -> Result<bool, CommandError> {
    let window_label = format!("ai-chat-{}", service_id);
    Ok(app.get_webview_window(&window_label).is_some())
}

/// Open an AI chat service URL in the default browser (alternative to WebView)
#[tauri::command]
#[specta::specta]
pub async fn open_ai_chat_in_browser(
    app: AppHandle,
    url: String,
) -> Result<bool, CommandError> {
    log::info!("🌍 open_ai_chat_in_browser called: {}", url);

    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| CommandError::infrastructure(format!("Failed to open URL: {}", e)))?;

    log::info!("✅ URL opened in browser: {}", url);
    Ok(true)
}
