#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod application;
mod presentation;

// Use external crates

use presentation::ipc;
use presentation::state::{AppState, CommandHandlers, Queries, Repositories, Services};
use std::time::Instant;
use tauri::{LogicalSize, Manager, Size, WindowEvent};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

const DEFAULT_WIDTH: f64 = 1200.0;
const DEFAULT_HEIGHT: f64 = 800.0;
const MIN_WIDTH: f64 = 800.0;
const MIN_HEIGHT: f64 = 600.0;

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("<non-string panic payload>");

        if let Some(location) = info.location() {
            eprintln!(
                "💥 panic at {}:{}:{}: {}",
                location.file(),
                location.line(),
                location.column(),
                payload
            );
            tracing::error!(
                "💥 panic at {}:{}:{}: {}",
                location.file(),
                location.line(),
                location.column(),
                payload
            );
        } else {
            eprintln!("💥 panic: {}", payload);
            tracing::error!("💥 panic: {}", payload);
        }
    }));
}

#[tokio::main]
async fn main() {
    let builder = ipc::builder();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .on_window_event(|window, event| match event {
            WindowEvent::Resized(size) => {
                let width = size.width as f64;
                let height = size.height as f64;

                if width >= MIN_WIDTH && height >= MIN_HEIGHT {
                    if let Err(e) = window.app_handle().save_window_state(StateFlags::all()) {
                        tracing::warn!("Failed to save window state: {}", e);
                    }
                } else {
                    tracing::debug!(
                        "Skipping window-state save for invalid size: {}x{}",
                        width, height
                    );
                }
            }
            WindowEvent::Moved(_) => {
                if let Err(e) = window.app_handle().save_window_state(StateFlags::POSITION) {
                    tracing::warn!("Failed to save window position: {}", e);
                }
            }
            WindowEvent::CloseRequested { .. } => {
                if let Err(e) = window.app_handle().save_window_state(StateFlags::all()) {
                    tracing::warn!("Failed to save window state on close: {}", e);
                }
            }
            _ => {}
        })
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            let handle = app.handle().clone();

            if let Some(main_window) = handle.get_webview_window("main") {
                if let Ok(size) = main_window.outer_size() {
                    let width = size.width as f64;
                    let height = size.height as f64;

                    if width < MIN_WIDTH || height < MIN_HEIGHT {
                        tracing::warn!(
                            "Restoring window to default size because persisted size {}x{} is below minimum",
                            width, height
                        );
                        if let Err(e) = main_window.set_size(Size::Logical(LogicalSize::new(
                            DEFAULT_WIDTH,
                            DEFAULT_HEIGHT,
                        ))) {
                            tracing::warn!("Failed to reset window size: {}", e);
                        } else if let Err(e) = main_window
                            .app_handle()
                            .save_window_state(StateFlags::all())
                        {
                            tracing::warn!("Failed to persist default window size: {}", e);
                        }
                    }
                }
            }

            // Initialize full logging system with file output
            let log_dir = handle
                .path()
                .app_log_dir()
                .map(|dir| dir.join("logs"))
                .unwrap_or_else(|e| {
                    eprintln!("⚠️  Failed to get app log directory: {}", e);
                    std::env::temp_dir().join("neuradock").join("logs")
                });

            match neuradock_infrastructure::logging::init_logger(log_dir.clone()) {
                Ok(_) => {
                    tracing::info!("🚀 NeuraDock starting...");
                    tracing::info!("📝 File logging initialized at: {}", log_dir.display());
                }
                Err(e) => {
                    eprintln!("⚠️  Failed to initialize file logging: {}", e);
                    eprintln!("   Falling back to console logging only");

                    let _ = tracing_subscriber::fmt()
                        .with_env_filter(
                            tracing_subscriber::EnvFilter::try_from_default_env()
                                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
                        )
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_line_number(true)
                        .try_init();
                }
            }

            install_panic_hook();

            // Initialize state and block startup until ready, so commands can't be invoked before
            // `AppState` is managed.
            let (tx, rx) = std::sync::mpsc::channel::<Result<AppState, String>>();
            let init_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                let result = AppState::new(init_handle).await.map_err(|e| e.to_string());
                let _ = tx.send(result);
            });

            tracing::info!("🚀 Starting app state initialization...");
            let started_at = Instant::now();
            let init_result = rx.recv().map_err(|e| {
                Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error>
            })?;
            match init_result {
                Ok(app_state) => {
                    let repositories: Repositories = app_state.repositories.clone();
                    let services: Services = app_state.services.clone();
                    let queries: Queries = app_state.queries.clone();
                    let command_handlers: CommandHandlers = app_state.command_handlers.clone();

                    app.manage(repositories);
                    app.manage(services);
                    app.manage(queries);
                    app.manage(command_handlers);
                    tracing::info!(
                        "✅ App state initialized successfully ({}ms)",
                        started_at.elapsed().as_millis()
                    );
                }
                Err(message) => {
                    tracing::error!("❌ Failed to initialize app state: {}", message);
                    return Err(Box::new(std::io::Error::other(message)));
                }
            }

            builder.mount_events(app);

            Ok(())
        })
        .run(tauri::generate_context!("../../tauri.conf.json"));

    if let Err(e) = app {
        eprintln!("❌ error while running tauri application: {}", e);
    }
}
