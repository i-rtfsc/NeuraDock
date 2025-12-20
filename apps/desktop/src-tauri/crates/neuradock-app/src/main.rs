#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod application;
mod presentation;

// Use external crates

use presentation::commands::*;
use presentation::ipc;
use presentation::state::AppState;
use tauri::Manager;

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
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            let handle = app.handle().clone();

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
            let init_result = rx.recv().map_err(|e| {
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
                    as Box<dyn std::error::Error>
            })?;
            match init_result {
                Ok(app_state) => {
                    app.manage(app_state);
                    tracing::info!("✅ App state initialized successfully");
                }
                Err(message) => {
                    tracing::error!("❌ Failed to initialize app state: {}", message);
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        message,
                    )));
                }
            }

            builder.mount_events(app);

            Ok(())
        })
        .run(tauri::generate_context!());

    if let Err(e) = app {
        eprintln!("❌ error while running tauri application: {}", e);
    }
}
