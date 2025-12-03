use crate::application::commands::account_commands::*;
use crate::application::commands::check_in_commands::*;
use crate::application::commands::notification_commands::*;
use crate::application::commands::command_handler::CommandHandler;
use crate::application::services::CheckInExecutor;
use crate::application::*;
use chrono::{Duration, Utc};
use neuradock_domain::account::{Account, Credentials};
use neuradock_domain::check_in::Provider;
use neuradock_domain::session::{Session, SessionRepository};
use neuradock_domain::shared::{AccountId, ProviderId};
use crate::presentation::state::AppState;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;

pub mod token;

// Re-export token commands
pub use token::*;

/// Get built-in providers
pub fn get_builtin_providers() -> HashMap<String, Provider> {
    let mut providers = HashMap::new();

    // AnyRouter provider
    providers.insert(
        "anyrouter".to_string(),
        Provider::builtin(
            "anyrouter",
            "AnyRouter".to_string(),
            "https://anyrouter.top".to_string(),
            "/login".to_string(),
            Some("/api/user/sign_in".to_string()),
            "/api/user/self".to_string(),
            Some("/api/token/".to_string()),
            "new-api-user".to_string(),
            Some("waf_cookies".to_string()),
        ),
    );

    // AgentRouter provider
    providers.insert(
        "agentrouter".to_string(),
        Provider::builtin(
            "agentrouter",
            "AgentRouter".to_string(),
            "https://agentrouter.org".to_string(),
            "/login".to_string(),
            None, // Auto check-in: balance updates when querying user info
            "/api/user/self".to_string(),
            Some("/api/token/".to_string()),
            "new-api-user".to_string(),
            None, // No WAF bypass needed
        ),
    );

    providers
}

#[tauri::command]
#[specta::specta]
pub async fn create_account(
    input: CreateAccountInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let command = CreateAccountCommand {
        name: input.name,
        provider_id: input.provider_id,
        cookies: input.cookies,
        api_user: input.api_user,
        auto_checkin_enabled: input.auto_checkin_enabled,
        auto_checkin_hour: input.auto_checkin_hour,
        auto_checkin_minute: input.auto_checkin_minute,
    };

    let result = state
        .command_handlers
        .create_account
        .handle(command)
        .await
        .map_err(|e| e.to_string())?;

    // Scheduler will be reloaded automatically via AccountCreated event
    // handled by SchedulerReloadEventHandler

    Ok(result.account_id)
}

#[tauri::command]
#[specta::specta]
pub async fn update_account(
    input: UpdateAccountInput,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let command = UpdateAccountCommand {
        account_id: input.account_id,
        name: input.name,
        cookies: input.cookies,
        api_user: input.api_user,
        auto_checkin_enabled: input.auto_checkin_enabled,
        auto_checkin_hour: input.auto_checkin_hour,
        auto_checkin_minute: input.auto_checkin_minute,
    };

    let result = state
        .command_handlers
        .update_account
        .handle(command)
        .await
        .map_err(|e| e.to_string())?;

    // Scheduler will be reloaded automatically via AccountUpdated event
    // handled by SchedulerReloadEventHandler

    Ok(result.success)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_account(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let command = DeleteAccountCommand { account_id };

    let result = state
        .command_handlers
        .delete_account
        .handle(command)
        .await
        .map_err(|e| e.to_string())?;

    // Scheduler will be reloaded automatically via AccountDeleted event
    // handled by SchedulerReloadEventHandler

    Ok(result.success)
}

#[tauri::command]
#[specta::specta]
pub async fn toggle_account(
    account_id: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let command = ToggleAccountCommand {
        account_id,
        enabled,
    };

    let result = state
        .command_handlers
        .toggle_account
        .handle(command)
        .await
        .map_err(|e| e.to_string())?;

    // Scheduler will be reloaded automatically via AccountToggled event
    // handled by SchedulerReloadEventHandler

    Ok(result.success)
}

#[tauri::command]
#[specta::specta]
pub async fn import_account_from_json(
    json_data: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let input: ImportAccountInput =
        serde_json::from_str(&json_data).map_err(|e| format!("Invalid JSON: {}", e))?;

    let cookies = input.cookies.clone();
    let credentials = Credentials::new(input.cookies, input.api_user);
    let account = Account::new(
        input.name,
        ProviderId::from_string(&input.provider),
        credentials,
    )
    .map_err(|e| e.to_string())?;

    let account_id = account.id().clone();

    state
        .account_repo
        .save(&account)
        .await
        .map_err(|e| e.to_string())?;

    // Create and save session with 30-day expiration
    const DEFAULT_SESSION_EXPIRATION_DAYS: i64 = 30;
    let session_token = cookies.values().next().cloned().unwrap_or_else(|| "session".to_string());
    let expires_at = Utc::now() + Duration::days(DEFAULT_SESSION_EXPIRATION_DAYS);
    let session = Session::new(account_id.clone(), session_token, expires_at)
        .map_err(|e| e.to_string())?;
    state
        .session_repo
        .save(&session)
        .await
        .map_err(|e| e.to_string())?;

    Ok(account_id.as_str().to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn import_accounts_batch(
    json_data: String,
    state: State<'_, AppState>,
) -> Result<BatchImportResult, String> {
    let inputs: Vec<ImportAccountInput> =
        serde_json::from_str(&json_data).map_err(|e| format!("Invalid JSON: {}", e))?;

    let mut results = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;

    for input in inputs {
        let account_name = input.name.clone();
        match import_single_account(input, &state.account_repo, &state.session_repo).await {
            Ok(account_id) => {
                succeeded += 1;
                results.push(ImportItemResult {
                    success: true,
                    account_id: Some(account_id),
                    account_name,
                    error: None,
                });
            }
            Err(e) => {
                failed += 1;
                results.push(ImportItemResult {
                    success: false,
                    account_id: None,
                    account_name,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(BatchImportResult {
        total: results.len() as i32,
        succeeded: succeeded as i32,
        failed: failed as i32,
        results,
    })
}

async fn import_single_account(
    input: ImportAccountInput,
    account_repo: &Arc<dyn crate::domain::account::AccountRepository>,
    session_repo: &Arc<dyn SessionRepository>,
) -> Result<String, Box<dyn std::error::Error>> {
    let cookies = input.cookies.clone();
    let credentials = Credentials::new(input.cookies, input.api_user);
    let account = Account::new(
        input.name,
        ProviderId::from_string(&input.provider),
        credentials,
    )?;

    let account_id = account.id().clone();
    account_repo.save(&account).await?;

    // Create and save session with 30-day expiration
    const DEFAULT_SESSION_EXPIRATION_DAYS: i64 = 30;
    let session_token = cookies.values().next().cloned().unwrap_or_else(|| "session".to_string());
    let expires_at = Utc::now() + Duration::days(DEFAULT_SESSION_EXPIRATION_DAYS);
    let session = Session::new(account_id.clone(), session_token, expires_at)?;
    session_repo.save(&session).await?;

    Ok(account_id.as_str().to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn export_accounts_to_json(
    input: ExportAccountsInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let accounts = if input.account_ids.is_empty() {
        state
            .account_repo
            .find_all()
            .await
            .map_err(|e| e.to_string())?
    } else {
        let mut result = Vec::new();
        for id_str in input.account_ids {
            let id = AccountId::from_string(&id_str);
            if let Some(account) = state
                .account_repo
                .find_by_id(&id)
                .await
                .map_err(|e| e.to_string())?
            {
                result.push(account);
            }
        }
        result
    };

    let export_data: Vec<serde_json::Value> = accounts
        .iter()
        .map(|acc| {
            let mut data = serde_json::json!({
                "name": acc.name(),
                "provider": acc.provider_id().as_str(),
            });

            if input.include_credentials {
                data["cookies"] = serde_json::to_value(acc.credentials().cookies()).unwrap();
                data["api_user"] =
                    serde_json::Value::String(acc.credentials().api_user().to_string());
            }

            data
        })
        .collect();

    serde_json::to_string_pretty(&export_data).map_err(|e| e.to_string())
}

/// Batch update accounts - match by name+provider, update cookies and api_user
/// If account doesn't exist and create_if_not_exists is true, create it
#[tauri::command]
#[specta::specta]
pub async fn update_accounts_batch(
    json_data: String,
    create_if_not_exists: bool,
    state: State<'_, AppState>,
) -> Result<BatchUpdateResult, String> {
    let inputs: Vec<ImportAccountInput> =
        serde_json::from_str(&json_data).map_err(|e| format!("Invalid JSON: {}", e))?;

    // Load all existing accounts for matching
    let existing_accounts = state
        .account_repo
        .find_all()
        .await
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    let mut updated = 0;
    let mut created = 0;
    let mut failed = 0;

    for input in inputs {
        let account_name = input.name.clone();
        let provider_id = input.provider.clone();

        // Try to find existing account by name + provider
        let existing = existing_accounts
            .iter()
            .find(|acc| acc.name() == input.name && acc.provider_id().as_str() == input.provider);

        match existing {
            Some(acc) => {
                // Update existing account
                let account_id = acc.id().clone();
                match update_account_cookies(
                    &account_id,
                    input.cookies,
                    input.api_user,
                    &state.account_repo,
                    &state.session_repo,
                )
                .await
                {
                    Ok(_) => {
                        updated += 1;
                        results.push(UpdateItemResult {
                            success: true,
                            account_id: Some(account_id.as_str().to_string()),
                            account_name,
                            action: "updated".to_string(),
                            error: None,
                        });
                    }
                    Err(e) => {
                        failed += 1;
                        results.push(UpdateItemResult {
                            success: false,
                            account_id: Some(account_id.as_str().to_string()),
                            account_name,
                            action: "failed".to_string(),
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
            None => {
                if create_if_not_exists {
                    // Create new account
                    match import_single_account(input, &state.account_repo, &state.session_repo).await {
                        Ok(account_id) => {
                            created += 1;
                            results.push(UpdateItemResult {
                                success: true,
                                account_id: Some(account_id),
                                account_name,
                                action: "created".to_string(),
                                error: None,
                            });
                        }
                        Err(e) => {
                            failed += 1;
                            results.push(UpdateItemResult {
                                success: false,
                                account_id: None,
                                account_name,
                                action: "failed".to_string(),
                                error: Some(e.to_string()),
                            });
                        }
                    }
                } else {
                    failed += 1;
                    results.push(UpdateItemResult {
                        success: false,
                        account_id: None,
                        account_name,
                        action: "failed".to_string(),
                        error: Some(format!("Account not found (provider: {})", provider_id)),
                    });
                }
            }
        }
    }

    // Scheduler will be reloaded automatically via domain events
    // (AccountCreated/AccountUpdated) handled by SchedulerReloadEventHandler

    Ok(BatchUpdateResult {
        total: results.len() as i32,
        updated: updated as i32,
        created: created as i32,
        failed: failed as i32,
        results,
    })
}

async fn update_account_cookies(
    account_id: &AccountId,
    cookies: HashMap<String, String>,
    api_user: String,
    account_repo: &Arc<dyn crate::domain::account::AccountRepository>,
    session_repo: &Arc<dyn SessionRepository>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut account = account_repo
        .find_by_id(account_id)
        .await?
        .ok_or("Account not found")?;

    let credentials = Credentials::new(cookies.clone(), api_user);
    account.update_credentials(credentials)?;
    account_repo.save(&account).await?;

    // Create and save session with 30-day expiration
    const DEFAULT_SESSION_EXPIRATION_DAYS: i64 = 30;
    let session_token = cookies.values().next().cloned().unwrap_or_else(|| "session".to_string());
    let expires_at = Utc::now() + Duration::days(DEFAULT_SESSION_EXPIRATION_DAYS);
    let session = Session::new(account_id.clone(), session_token, expires_at)?;
    session_repo.save(&session).await?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn execute_check_in(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<ExecuteCheckInResult, String> {
    use crate::application::commands::check_in_commands::{ExecuteCheckInCommand, CheckInCommandResult};

    log::info!(
        "=== execute_check_in command called for account: {} ===",
        account_id
    );

    let command = ExecuteCheckInCommand {
        account_id: account_id.clone(),
    };

    let result = state
        .command_handlers
        .execute_check_in
        .handle(command)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ExecuteCheckInResult {
        job_id: account_id,
        success: result.success,
        balance: result.balance,
        error: if result.success {
            None
        } else {
            Some(result.message)
        },
    })
}

#[tauri::command]
#[specta::specta]
pub async fn execute_batch_check_in(
    account_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<BatchCheckInResult, String> {
    use crate::application::commands::check_in_commands::{BatchExecuteCheckInCommand, BatchCheckInCommandResult};

    let command = BatchExecuteCheckInCommand { account_ids };

    let result = state
        .command_handlers
        .batch_execute_check_in
        .handle(command)
        .await
        .map_err(|e| e.to_string())?;

    // Convert results to DTOs
    let results_dto: Vec<ExecuteCheckInResult> = result
        .results
        .into_iter()
        .map(|r| ExecuteCheckInResult {
            job_id: format!("batch_{}", r.message.len()), // Placeholder
            success: r.success,
            balance: r.balance,
            error: if r.success {
                None
            } else {
                Some(r.message)
            },
        })
        .collect();

    Ok(BatchCheckInResult {
        total: result.total as i32,
        succeeded: result.succeeded as i32,
        failed: result.failed as i32,
        results: results_dto,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn stop_check_in(job_id: String, state: State<'_, AppState>) -> Result<bool, String> {
    Err("Not implemented yet".to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn add_provider(
    input: AddProviderInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    Err("Not implemented yet".to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn check_browser_available() -> Result<BrowserInfoDto, String> {
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

#[tauri::command]
#[specta::specta]
pub async fn get_all_providers(state: State<'_, AppState>) -> Result<Vec<ProviderDto>, String> {
    let providers = get_builtin_providers();
    let accounts = state
        .account_repo
        .find_all()
        .await
        .map_err(|e| e.to_string())?;

    let dtos: Vec<ProviderDto> = providers
        .iter()
        .map(|(id, provider)| {
            let account_count = accounts
                .iter()
                .filter(|acc| acc.provider_id().as_str() == id)
                .count();

            ProviderDto {
                id: id.clone(),
                name: provider.name().to_string(),
                domain: provider.domain().to_string(),
                is_builtin: provider.is_builtin(),
                account_count: account_count as i32,
            }
        })
        .collect();

    Ok(dtos)
}

#[tauri::command]
#[specta::specta]
pub async fn get_all_accounts(
    enabled_only: bool,
    state: State<'_, AppState>,
) -> Result<Vec<AccountDto>, String> {
    let providers = get_builtin_providers();
    
    state
        .account_queries
        .get_all_accounts(enabled_only, &providers)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_account_detail(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<AccountDetailDto, String> {
    let id = AccountId::from_string(&account_id);
    let account = state
        .account_repo
        .find_by_id(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Account not found")?;

    let providers = get_builtin_providers();
    let provider_name = providers
        .get(account.provider_id().as_str())
        .map(|p| p.name().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    Ok(AccountDetailDto {
        id: account.id().as_str().to_string(),
        name: account.name().to_string(),
        provider_id: account.provider_id().as_str().to_string(),
        provider_name,
        api_user: account.credentials().api_user().to_string(),
        cookies: account.credentials().cookies().clone(),
        cookies_count: account.credentials().cookies().len() as i32,
        enabled: account.is_enabled(),
        last_check_in: account.last_check_in().map(|dt| dt.to_rfc3339()),
        last_balance: None,
        created_at: account.created_at().to_rfc3339(),
        auto_checkin_enabled: account.auto_checkin_enabled(),
        auto_checkin_hour: account.auto_checkin_hour(),
        auto_checkin_minute: account.auto_checkin_minute(),
    })
}

#[tauri::command]
#[specta::specta]
pub async fn get_check_in_history(
    account_id: Option<String>,
    page: i32,
    page_size: i32,
    state: State<'_, AppState>,
) -> Result<Vec<CheckInHistoryDto>, String> {
    Err("Not implemented yet".to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_check_in_stats(
    account_id: Option<String>,
    period: String,
    state: State<'_, AppState>,
) -> Result<CheckInStatsDto, String> {
    Err("Not implemented yet".to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_running_jobs(state: State<'_, AppState>) -> Result<Vec<RunningJobDto>, String> {
    Err("Not implemented yet".to_string())
}

// Fetch account balance with smart caching
// Only performs fresh login if cache is stale (> 1 hour) or no cache exists
// Set force_refresh to true to ignore cache and always fetch fresh balance
#[tauri::command]
#[specta::specta]
pub async fn fetch_account_balance(
    account_id: String,
    force_refresh: Option<bool>,
    state: State<'_, AppState>,
) -> Result<BalanceDto, String> {
    const MAX_CACHE_AGE_HOURS: i64 = 1;

    let providers = get_builtin_providers();
    let force_refresh = force_refresh.unwrap_or(false);

    // Get account
    let acc_id = AccountId::from_string(&account_id);
    let mut account = state
        .account_repo
        .find_by_id(&acc_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Account not found")?;

    // Check if we have valid cached balance (unless force_refresh is true)
    if !force_refresh && !account.is_balance_stale(MAX_CACHE_AGE_HOURS) {
        // Use cached balance
        if let (Some(current_balance), Some(total_consumed), Some(total_income)) = (
            account.current_balance(),
            account.total_consumed(),
            account.total_income(),
        ) {
            return Ok(BalanceDto {
                current_balance,
                total_consumed,
                total_income,
            });
        }
    }

    // Cache is stale or doesn't exist, or force_refresh is true - fetch fresh balance
    let provider_id = account.provider_id().as_str();
    let provider = providers
        .get(provider_id)
        .ok_or(format!("Provider {} not found", provider_id))?;

    // Create executor
    let executor =
        CheckInExecutor::new(state.account_repo.clone(), true).map_err(|e| e.to_string())?;

    // Fetch balance only (without triggering check-in)
    let user_info = executor
        .fetch_balance_only(&account_id, provider)
        .await
        .map_err(|e| e.to_string())?;

    // Note: API returns quota (current balance) and used_quota (total consumed)
    let current_balance = user_info.quota;
    let total_consumed = user_info.used_quota;
    let balance_dto = BalanceDto {
        current_balance,
        total_consumed,
        total_income: current_balance + total_consumed, // Total income = current balance + consumed
    };

    // Update account cache
    account.update_balance(
        balance_dto.current_balance,
        balance_dto.total_consumed,
        balance_dto.total_income,
    );
    state
        .account_repo
        .save(&account)
        .await
        .map_err(|e| e.to_string())?;

    // Store balance history (only if significantly changed or first time)
    save_balance_history(&account_id, &balance_dto, &state).await?;

    Ok(balance_dto)
}

// Fetch balances for multiple accounts
#[tauri::command]
#[specta::specta]
pub async fn fetch_accounts_balances(
    account_ids: Vec<String>,
    force_refresh: Option<bool>,
    state: State<'_, AppState>,
) -> Result<HashMap<String, Option<BalanceDto>>, String> {
    let mut results = HashMap::new();

    for account_id in account_ids {
        match fetch_account_balance(account_id.clone(), force_refresh, state.clone()).await {
            Ok(balance) => {
                results.insert(account_id, Some(balance));
            }
            Err(_) => {
                results.insert(account_id, None);
            }
        }
    }

    Ok(results)
}

// Get balance statistics by provider
#[tauri::command]
#[specta::specta]
pub async fn get_balance_statistics(
    state: State<'_, AppState>,
) -> Result<BalanceStatisticsDto, String> {
    let pool = &*state.pool;

    // Get all accounts with their latest balances
    let accounts = state
        .account_repo
        .find_enabled()
        .await
        .map_err(|e| e.to_string())?;
    let providers = get_builtin_providers();

    let mut provider_stats: HashMap<String, ProviderBalanceDto> = HashMap::new();
    let mut total_current_balance = 0.0;
    let mut total_consumed = 0.0;
    let mut total_income = 0.0;

    for account in accounts {
        // Use cached balance from account directly (faster than querying balance_history)
        let balance_opt = match (
            account.current_balance(),
            account.total_consumed(),
            account.total_income(),
        ) {
            (Some(cb), Some(tc), Some(ti)) => Some((cb, tc, ti)),
            _ => {
                // Fallback to balance_history if account cache is empty
                let account_id = account.id().as_str();
                sqlx::query_as::<_, (f64, f64, f64)>(
                    "SELECT current_balance, total_consumed, total_income FROM balance_history
                     WHERE account_id = ? ORDER BY recorded_at DESC LIMIT 1",
                )
                .bind(account_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?
            }
        };

        if let Some((current_balance, consumed, income)) = balance_opt {
            let provider_id = account.provider_id().as_str();
            let provider_name = providers
                .get(provider_id)
                .map(|p| p.name().to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let stat =
                provider_stats
                    .entry(provider_id.to_string())
                    .or_insert(ProviderBalanceDto {
                        provider_id: provider_id.to_string(),
                        provider_name,
                        current_balance: 0.0,
                        total_consumed: 0.0,
                        total_income: 0.0,
                        account_count: 0,
                    });

            stat.current_balance += current_balance;
            stat.total_consumed += consumed;
            stat.total_income += income;
            stat.account_count += 1;

            total_current_balance += current_balance;
            total_consumed += consumed;
            total_income += income;
        }
    }

    Ok(BalanceStatisticsDto {
        providers: provider_stats.into_iter().map(|(_, v)| v).collect(),
        total_current_balance,
        total_consumed,
        total_income,
    })
}

// Helper function to save balance history
async fn save_balance_history(
    account_id: &str,
    balance: &BalanceDto,
    state: &State<'_, AppState>,
) -> Result<(), String> {
    let pool = &*state.pool;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    // Check if we already have a record today
    let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
    let today_start_str =
        chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(today_start, chrono::Utc)
            .to_rfc3339();

    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM balance_history WHERE account_id = ? AND recorded_at >= ? LIMIT 1",
    )
    .bind(account_id)
    .bind(&today_start_str)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    // Only insert if no record exists for today or if values changed significantly
    if existing.is_none() {
        sqlx::query(
            "INSERT INTO balance_history (id, account_id, current_balance, total_consumed, total_income, recorded_at)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(account_id)
        .bind(balance.current_balance)
        .bind(balance.total_consumed)
        .bind(balance.total_income)
        .bind(now.to_rfc3339())
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

// ============================================================
// Check-in Streak Commands
// ============================================================

#[tauri::command]
#[specta::specta]
pub async fn get_check_in_streak(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<dtos::CheckInStreakDto, String> {
    state
        .streak_queries
        .get_streak_stats(&account_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_all_check_in_streaks(
    state: State<'_, AppState>,
) -> Result<Vec<dtos::CheckInStreakDto>, String> {
    state
        .streak_queries
        .get_all_streaks()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_check_in_calendar(
    account_id: String,
    year: i32,
    month: u32,
    state: State<'_, AppState>,
) -> Result<dtos::CheckInCalendarDto, String> {
    state
        .streak_queries
        .get_calendar(&account_id, year, month)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_check_in_trend(
    account_id: String,
    days: u32,
    state: State<'_, AppState>,
) -> Result<dtos::CheckInTrendDto, String> {
    state
        .streak_queries
        .get_trend(&account_id, days)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_check_in_day_detail(
    account_id: String,
    date: String,
    state: State<'_, AppState>,
) -> Result<dtos::CheckInDayDto, String> {
    state
        .streak_queries
        .get_day_detail(&account_id, &date)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn recalculate_check_in_streaks(state: State<'_, AppState>) -> Result<(), String> {
    state
        .streak_queries
        .recalculate_all_streaks()
        .await
        .map_err(|e| e.to_string())
}

// ===== Config Commands =====

#[tauri::command]
#[specta::specta]
pub async fn get_log_level(state: State<'_, AppState>) -> Result<String, String> {
    let level = state.config_service.get_log_level();
    Ok(level.as_str().to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn set_log_level(level: String, state: State<'_, AppState>) -> Result<(), String> {
    use crate::application::services::LogLevel;

    let log_level = match level.to_lowercase().as_str() {
        "error" => LogLevel::Error,
        "warn" => LogLevel::Warn,
        "info" => LogLevel::Info,
        "debug" => LogLevel::Debug,
        "trace" => LogLevel::Trace,
        _ => return Err("Invalid log level. Must be one of: error, warn, info, debug, trace".to_string()),
    };

    state.config_service.set_log_level(log_level)
        .map_err(|e| format!("Failed to save log level: {}", e))?;
    Ok(())
}

// ============================================================
// Notification Commands
// ============================================================

#[tauri::command]
#[specta::specta]
pub async fn create_notification_channel(
    input: CreateNotificationChannelInput,
    state: State<'_, AppState>,
) -> Result<NotificationChannelDto, String> {
    use crate::application::commands::notification_commands::CreateNotificationChannelCommand;

    let command = CreateNotificationChannelCommand { input };

    state
        .command_handlers
        .create_notification_channel
        .handle(command)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn update_notification_channel(
    input: UpdateNotificationChannelInput,
    state: State<'_, AppState>,
) -> Result<NotificationChannelDto, String> {
    use crate::application::commands::notification_commands::UpdateNotificationChannelCommand;

    let command = UpdateNotificationChannelCommand { input };

    state
        .command_handlers
        .update_notification_channel
        .handle(command)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_notification_channel(
    channel_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use crate::application::commands::notification_commands::DeleteNotificationChannelCommand;

    let command = DeleteNotificationChannelCommand { channel_id };

    state
        .command_handlers
        .delete_notification_channel
        .handle(command)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_all_notification_channels(
    state: State<'_, AppState>,
) -> Result<Vec<NotificationChannelDto>, String> {
    let channels = state
        .notification_channel_repo
        .find_all()
        .await
        .map_err(|e| e.to_string())?;

    let dtos = channels
        .iter()
        .map(|channel| NotificationChannelDto {
            id: channel.id().as_str().to_string(),
            channel_type: channel.channel_type().as_str().to_string(),
            config: serde_json::to_value(channel.config()).unwrap_or(serde_json::json!({})),
            enabled: channel.is_enabled(),
            created_at: channel.created_at().to_rfc3339(),
        })
        .collect();

    Ok(dtos)
}

#[tauri::command]
#[specta::specta]
pub async fn test_notification_channel(
    channel_id: String,
    state: State<'_, AppState>,
) -> Result<TestNotificationChannelResult, String> {
    use crate::application::commands::notification_commands::TestNotificationChannelCommand;

    let command = TestNotificationChannelCommand { channel_id };

    state
        .command_handlers
        .test_notification_channel
        .handle(command)
        .await
        .map_err(|e| e.to_string())
}
