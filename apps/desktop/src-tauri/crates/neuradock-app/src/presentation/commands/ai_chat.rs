use crate::application::dtos::{
    AiChatServiceDto, CreateAiChatServiceInput, ReorderAiChatServicesInput,
    UpdateAiChatServiceInput,
};
use crate::presentation::error::CommandError;
use crate::presentation::state::Repositories;
use neuradock_domain::ai_chat::{AiChatService, AiChatServiceId};
use std::collections::VecDeque;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, Position, Size, State, WebviewBuilder, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

/// State to track multiple embedded AI chat webviews with LRU caching
pub struct EmbeddedAiChatState {
    /// LRU queue of service_ids (most recent at back)
    lru_queue: Mutex<VecDeque<String>>,
    /// Currently visible webview's service_id
    visible_service_id: Mutex<Option<String>>,
}

impl Default for EmbeddedAiChatState {
    fn default() -> Self {
        Self {
            lru_queue: Mutex::new(VecDeque::new()),
            visible_service_id: Mutex::new(None),
        }
    }
}

fn webview_label_for_service(service_id: &str) -> String {
    format!("ai-chat-{}", service_id)
}

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

    // Use different label prefix for independent windows vs embedded webviews
    let window_label = format!("ai-chat-window-{}", service_id);

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

    let window_label = format!("ai-chat-window-{}", service_id);

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

    let window_label = format!("ai-chat-window-{}", service_id);

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
    let window_label = format!("ai-chat-window-{}", service_id);
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

// ============================================================================
// Embedded WebView Commands (for inline display in main window)
// Multi-webview support: each tab has its own persistent webview
// ============================================================================

/// Show an embedded AI chat webview within the main window at specified position
/// Creates a new webview if not exists, otherwise shows existing one
/// Uses LRU caching to limit memory usage
#[tauri::command]
#[specta::specta]
pub async fn show_embedded_ai_chat(
    app: AppHandle,
    embedded_state: State<'_, EmbeddedAiChatState>,
    service_id: String,
    url: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    max_cached: u32,
) -> Result<bool, CommandError> {
    log::info!(
        "🌐 show_embedded_ai_chat called: {} at ({}, {}) {}x{} (max_cached: {})",
        service_id, x, y, width, height, max_cached
    );

    let main_window = app
        .get_window("main")
        .ok_or_else(|| CommandError::infrastructure("Main window not found"))?;

    let webview_label = webview_label_for_service(&service_id);
    
    // Hide the currently visible webview (if different from target)
    {
        let visible_id = embedded_state.visible_service_id.lock().unwrap().clone();
        if let Some(ref vid) = visible_id {
            if vid != &service_id {
                let old_label = webview_label_for_service(vid);
                if let Some(old_webview) = app.get_webview(&old_label) {
                    // Move off-screen to hide (Tauri doesn't have hide/show for child webviews)
                    let _ = old_webview.set_position(Position::Logical(tauri::LogicalPosition::new(-10000.0, -10000.0)));
                    log::info!("🙈 Hid webview for: {}", vid);
                }
            }
        }
    }

    // Check if webview already exists for this service
    let webview_exists = {
        let lru = embedded_state.lru_queue.lock().unwrap();
        lru.contains(&service_id)
    };

    if webview_exists {
        // Webview exists, just reposition and show it
        if let Some(webview) = app.get_webview(&webview_label) {
            webview
                .set_position(Position::Logical(tauri::LogicalPosition::new(x, y)))
                .map_err(|e| CommandError::infrastructure(format!("Failed to set webview position: {}", e)))?;
            webview
                .set_size(Size::Logical(tauri::LogicalSize::new(width, height)))
                .map_err(|e| CommandError::infrastructure(format!("Failed to set webview size: {}", e)))?;
            log::info!("👁️ Showing existing webview for: {}", service_id);
        }
        
        // Move to end of LRU queue (most recently used)
        {
            let mut lru = embedded_state.lru_queue.lock().unwrap();
            lru.retain(|id| id != &service_id);
            lru.push_back(service_id.clone());
        }
    } else {
        // Evict oldest webviews if we're at capacity
        {
            let mut lru = embedded_state.lru_queue.lock().unwrap();
            while lru.len() >= max_cached as usize {
                if let Some(oldest_id) = lru.pop_front() {
                    let oldest_label = webview_label_for_service(&oldest_id);
                    if let Some(old_webview) = app.get_webview(&oldest_label) {
                        let _ = old_webview.close();
                        log::info!("🗑️ Evicted oldest webview: {}", oldest_id);
                    }
                }
            }
        }
        
        // Create new webview for this service
        let parsed_url: url::Url = url.parse().map_err(|e| {
            CommandError::validation(format!("Invalid URL: {}", e))
        })?;

        log::info!("🆕 Creating webview for: {} (label: {})", service_id, webview_label);
        let _webview = main_window
            .add_child(
                WebviewBuilder::new(&webview_label, WebviewUrl::External(parsed_url))
                    .auto_resize(),
                Position::Logical(tauri::LogicalPosition::new(x, y)),
                Size::Logical(tauri::LogicalSize::new(width, height)),
            )
            .map_err(|e| {
                CommandError::infrastructure(format!("Failed to create embedded webview: {}", e))
            })?;

        // Add to LRU queue
        {
            let mut lru = embedded_state.lru_queue.lock().unwrap();
            lru.push_back(service_id.clone());
        }

        log::info!("✅ Webview created for: {}", service_id);
    }

    // Update visible service
    *embedded_state.visible_service_id.lock().unwrap() = Some(service_id);

    Ok(true)
}

/// Hide the currently visible embedded AI chat webview (move off-screen)
#[tauri::command]
#[specta::specta]
pub async fn hide_embedded_ai_chat(
    app: AppHandle,
    embedded_state: State<'_, EmbeddedAiChatState>,
) -> Result<bool, CommandError> {
    log::info!("🙈 hide_embedded_ai_chat called");

    let visible_id = embedded_state.visible_service_id.lock().unwrap().clone();
    if let Some(ref vid) = visible_id {
        let label = webview_label_for_service(vid);
        if let Some(webview) = app.get_webview(&label) {
            // Move off-screen to hide
            let _ = webview.set_position(Position::Logical(tauri::LogicalPosition::new(-10000.0, -10000.0)));
            log::info!("✅ Webview hidden for: {}", vid);
        }
    }

    // Clear visible state
    *embedded_state.visible_service_id.lock().unwrap() = None;

    Ok(true)
}

/// Close and remove a specific embedded AI chat webview (when tab is closed)
#[tauri::command]
#[specta::specta]
pub async fn close_embedded_ai_chat(
    app: AppHandle,
    embedded_state: State<'_, EmbeddedAiChatState>,
    service_id: String,
) -> Result<bool, CommandError> {
    log::info!("🗑️ close_embedded_ai_chat called for: {}", service_id);

    let label = webview_label_for_service(&service_id);
    if let Some(webview) = app.get_webview(&label) {
        webview.close().map_err(|e| {
            CommandError::infrastructure(format!("Failed to close webview: {}", e))
        })?;
        log::info!("✅ Webview closed for: {}", service_id);
    }

    // Remove from LRU queue
    {
        let mut lru = embedded_state.lru_queue.lock().unwrap();
        lru.retain(|id| id != &service_id);
    }

    // Clear visible if this was visible
    {
        let mut visible = embedded_state.visible_service_id.lock().unwrap();
        if visible.as_ref() == Some(&service_id) {
            *visible = None;
        }
    }

    Ok(true)
}

/// Close all embedded AI chat webviews (when navigating away from AI Chat page)
#[tauri::command]
#[specta::specta]
pub async fn close_all_embedded_ai_chats(
    app: AppHandle,
    embedded_state: State<'_, EmbeddedAiChatState>,
) -> Result<bool, CommandError> {
    log::info!("🗑️ close_all_embedded_ai_chats called");

    let service_ids: Vec<String> = {
        let lru = embedded_state.lru_queue.lock().unwrap();
        lru.iter().cloned().collect()
    };

    for service_id in service_ids {
        let label = webview_label_for_service(&service_id);
        if let Some(webview) = app.get_webview(&label) {
            let _ = webview.close();
            log::info!("✅ Closed webview for: {}", service_id);
        }
    }

    // Clear all state
    embedded_state.lru_queue.lock().unwrap().clear();
    *embedded_state.visible_service_id.lock().unwrap() = None;

    Ok(true)
}

/// Refresh the currently visible embedded AI chat webview
#[tauri::command]
#[specta::specta]
pub async fn refresh_embedded_ai_chat(
    app: AppHandle,
    embedded_state: State<'_, EmbeddedAiChatState>,
) -> Result<bool, CommandError> {
    log::info!("🔄 refresh_embedded_ai_chat called");

    let visible_id = embedded_state.visible_service_id.lock().unwrap().clone();
    if let Some(ref vid) = visible_id {
        let label = webview_label_for_service(vid);
        if let Some(webview) = app.get_webview(&label) {
            webview
                .eval("location.reload()")
                .map_err(|e| {
                    CommandError::infrastructure(format!("Failed to refresh webview: {}", e))
                })?;
            log::info!("✅ Webview refreshed for: {}", vid);
            return Ok(true);
        }
    }

    log::warn!("⚠️ No visible webview to refresh");
    Ok(false)
}
