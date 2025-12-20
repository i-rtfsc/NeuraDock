use tauri_specta::{collect_commands, collect_events, Builder};

pub fn builder() -> Builder<tauri::Wry> {
    #[allow(unused_imports)]
    use crate::*;
    use crate::presentation::commands::*;

    Builder::<tauri::Wry>::new()
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
            create_provider,
            update_provider,
            delete_provider,
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
            // Notification commands
            create_notification_channel,
            update_notification_channel,
            delete_notification_channel,
            get_all_notification_channels,
            test_notification_channel,
            // Token commands
            fetch_account_tokens,
            configure_claude_global,
            generate_claude_temp_commands,
            configure_codex_global,
            generate_codex_temp_commands,
            check_model_compatibility,
            get_provider_nodes,
            add_custom_node,
            delete_custom_node,
            clear_claude_global,
            clear_codex_global,
            fetch_provider_models,
            refresh_provider_models_with_waf,
            get_cached_provider_models,
            // Independent API Key commands
            get_all_independent_keys,
            get_independent_key_by_id,
            create_independent_key,
            update_independent_key,
            delete_independent_key,
            toggle_independent_key,
            configure_independent_key_claude,
            generate_independent_key_claude_temp,
            configure_independent_key_codex,
            generate_independent_key_codex_temp,
            // System & Logging commands
            get_app_version,
            log_from_frontend,
            open_log_dir,
        ])
        .events(collect_events![
            crate::presentation::events::CheckInProgress,
            crate::presentation::events::BalanceUpdated,
        ])
}

