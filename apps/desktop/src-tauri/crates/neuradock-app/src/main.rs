#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod application;
mod presentation;

// Use external crates
use neuradock_domain as domain;
use neuradock_infrastructure as infrastructure;

use presentation::commands::*;
use presentation::state::AppState;
use specta_typescript::Typescript;
use std::sync::Arc;
use tauri::Manager;
use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_specta::{collect_commands, collect_events, Builder};
use tokio::sync::OnceCell;

// Global state cell to ensure initialization completes before use
static STATE_CELL: OnceCell<()> = OnceCell::const_new();

#[tokio::main]
async fn main() {
    // Initialize tracing with env filter
    // Set RUST_LOG=debug for detailed logs, or RUST_LOG=info for normal operation
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();
    
    tracing::info!("🚀 NeuraDock starting...");

    let builder = Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            // Account commands
            create_account,
            update_account,
            delete_account,
            toggle_account,
            import_account_from_json,
            import_accounts_batch,
            update_accounts_batch,
            export_accounts_to_json,
            // Check-in commands
            execute_check_in,
            execute_batch_check_in,
            stop_check_in,
            // Balance commands
            fetch_account_balance,
            fetch_accounts_balances,
            get_balance_statistics,
            // Provider commands
            add_provider,
            get_all_providers,
            // Query commands
            get_all_accounts,
            get_account_detail,
            get_check_in_history,
            get_check_in_stats,
            get_running_jobs,
            // Check-in Streak commands
            get_check_in_streak,
            get_all_check_in_streaks,
            get_check_in_calendar,
            get_check_in_trend,
            get_check_in_day_detail,
            recalculate_check_in_streaks,
            // Config commands
            get_log_level,
            set_log_level,
        ])
        .events(collect_events![
            presentation::events::CheckInProgress,
            presentation::events::BalanceUpdated,
        ]);

    #[cfg(debug_assertions)]
    {
        // Try to export TypeScript bindings, but don't fail if it errors
        let _ = builder.export(Typescript::default(), "../src/lib/tauri.ts");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            let handle = app.handle().clone();

            // Initialize state asynchronously
            tauri::async_runtime::spawn(async move {
                tracing::info!("🚀 Starting app state initialization...");
                match AppState::new(handle.clone()).await {
                    Ok(app_state) => {
                        handle.manage(app_state);
                        STATE_CELL.set(()).ok();
                        tracing::info!("✅ App state initialized successfully");
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to initialize app state: {}", e);
                    }
                }
            });

            builder.mount_events(app);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
