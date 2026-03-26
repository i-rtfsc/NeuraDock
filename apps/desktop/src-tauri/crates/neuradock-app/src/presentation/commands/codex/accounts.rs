/// Codex account management commands

use chrono::Utc;
use neuradock_domain::codex::CodexAccount;
use neuradock_infrastructure::codex_auth::{CodexAuthFile, CodexAuthJson};
use neuradock_infrastructure::http::openai::quota::{fetch_codex_usage, CodexUsageQuota};
use tauri::State;
use tracing::{info, warn};

use crate::application::dtos::{CodexAccountDto, CodexAuthInfoDto, CodexInboxCodeDto, CodexQuotaDto, CodexRateLimitWindowDto};
use crate::presentation::error::CommandError;
use crate::presentation::state::Repositories;
use super::quota::{apply_usage_quota, quota_to_dto};
use neuradock_domain::codex::CodexAccountId;
use neuradock_infrastructure::http::openai::tempmail::TempmailClient;

fn map_err(e: impl std::fmt::Display) -> CommandError {
    CommandError::from(e.to_string())
}

async fn find_linked_account(
    repos: &Repositories,
    email: Option<&str>,
    account_id: Option<&str>,
) -> Result<Option<neuradock_domain::codex::CodexAccount>, CommandError> {
    if let Some(account_id) = account_id.filter(|value| !value.trim().is_empty()) {
        let accounts = repos.codex_account.find_all().await.map_err(map_err)?;
        if let Some(account) = accounts
            .into_iter()
            .find(|account| account.account_id() == Some(account_id))
        {
            return Ok(Some(account));
        }
    }

    match email.filter(|value| !value.trim().is_empty()) {
        Some(email) => repos.codex_account.find_by_email(email).await.map_err(map_err),
        None => Ok(None),
    }
}

async fn sync_active_auth_quota(
    repos: &Repositories,
    email: Option<&str>,
    account_id: Option<&str>,
    quota: &CodexUsageQuota,
) -> Result<(), CommandError> {
    let Some(mut account) = find_linked_account(repos, email, account_id).await? else {
        return Ok(());
    };

    apply_usage_quota(&mut account, quota);
    repos.codex_account.save(&account).await.map_err(map_err)
}

fn map_cached_window(
    window: Option<&neuradock_domain::codex::CodexRateLimitWindow>,
) -> Option<CodexRateLimitWindowDto> {
    window.map(|window| CodexRateLimitWindowDto {
        used_percent: window.used_percent(),
        window_minutes: window.window_minutes(),
        resets_at: window.resets_at().map(|dt| dt.to_rfc3339()),
    })
}

fn quota_from_linked_account(
    account: Option<&neuradock_domain::codex::CodexAccount>,
) -> Option<CodexQuotaDto> {
    let account = account?;
    let quota = CodexQuotaDto {
        plan_type: account.plan_type().map(|value| value.to_string()),
        has_credits: account.has_credits(),
        is_unlimited: account.is_unlimited(),
        credit_balance: account.credit_balance().map(|value| value.to_string()),
        primary_window: map_cached_window(account.primary_window()),
        secondary_window: map_cached_window(account.secondary_window()),
    };

    if quota.plan_type.is_none()
        && quota.has_credits.is_none()
        && quota.is_unlimited.is_none()
        && quota.credit_balance.is_none()
        && quota.primary_window.is_none()
        && quota.secondary_window.is_none()
    {
        None
    } else {
        Some(quota)
    }
}

fn resolve_active_auth_last_refresh(
    auth: &CodexAuthJson,
    linked_account: Option<&CodexAccount>,
    cached_refresh_at: Option<&str>,
) -> String {
    linked_account
        .and_then(|account| account.quota_checked_at().map(|dt| dt.to_rfc3339()))
        .or_else(|| {
            cached_refresh_at
                .filter(|value| !value.trim().is_empty())
                .map(|value| value.to_string())
        })
        .unwrap_or_else(|| auth.last_refresh.clone())
}

async fn load_active_auth(
    repos: &Repositories,
    refresh_quota: bool,
) -> Result<Option<CodexAuthInfoDto>, CommandError> {
    let auth = CodexAuthFile::read().map_err(map_err)?;

    Ok(match auth {
        Some(a) => {
            let email = a.email();
            let account_id = a.tokens.as_ref().map(|t| t.account_id.clone());
            let has_tokens = a.tokens.is_some()
                && !a
                    .tokens
                    .as_ref()
                    .map(|t| t.access_token.is_empty())
                    .unwrap_or(true);

            let linked_account =
                find_linked_account(repos, email.as_deref(), account_id.as_deref()).await?;
            let quota_cache = match CodexAuthFile::read_quota_cache(&a) {
                Ok(cache) => cache,
                Err(error) => {
                    warn!("Failed to read active Codex auth quota cache: {}", error);
                    None
                }
            };
            let cached_quota = quota_from_linked_account(linked_account.as_ref())
                .or_else(|| quota_cache.as_ref().map(|(quota, _)| quota_to_dto(quota)));
            let mut last_refresh = resolve_active_auth_last_refresh(
                &a,
                linked_account.as_ref(),
                quota_cache.as_ref().map(|(_, refreshed_at)| refreshed_at.as_str()),
            );

            let (quota, quota_error) = if refresh_quota && has_tokens {
                if let Some(tokens) = a.tokens.as_ref() {
                    match fetch_codex_usage(
                        &tokens.access_token,
                        (!tokens.account_id.is_empty()).then_some(tokens.account_id.as_str()),
                    )
                    .await
                    {
                        Ok(quota) => {
                            let refreshed_at = Utc::now().to_rfc3339();
                            if let Err(error) = sync_active_auth_quota(
                                repos,
                                email.as_deref(),
                                (!tokens.account_id.is_empty()).then_some(tokens.account_id.as_str()),
                                &quota,
                            )
                            .await
                            {
                                warn!("Failed to sync active Codex auth quota to account store: {}", error);
                            }
                            if let Err(error) =
                                CodexAuthFile::write_quota_cache(&a, &quota, &refreshed_at)
                            {
                                warn!("Failed to persist active Codex auth quota cache: {}", error);
                            }
                            last_refresh = refreshed_at;
                            (Some(quota_to_dto(&quota)), None)
                        }
                        Err(error) => {
                            warn!("Failed to fetch active Codex auth quota: {}", error);
                            (cached_quota, Some(error.to_string()))
                        }
                    }
                } else {
                    (cached_quota, None)
                }
            } else {
                (cached_quota, None)
            };

            Some(CodexAuthInfoDto {
                auth_mode: a.auth_mode.clone(),
                email,
                openai_api_key: a.openai_api_key.clone(),
                account_id,
                last_refresh,
                has_tokens,
                quota,
                quota_error,
            })
        }
        None => None,
    })
}

// ─────────────────────────── Account CRUD ────────────────────────────

#[tauri::command]
#[specta::specta]
pub async fn list_codex_accounts(
    repos: State<'_, Repositories>,
) -> Result<Vec<CodexAccountDto>, CommandError> {
    let accounts = repos.codex_account.find_all().await.map_err(map_err)?;
    Ok(accounts.iter().map(CodexAccountDto::from).collect())
}

#[tauri::command]
#[specta::specta]
pub async fn get_codex_account(
    id: String,
    repos: State<'_, Repositories>,
) -> Result<Option<CodexAccountDto>, CommandError> {
    let account_id = CodexAccountId::from_string(&id);
    let account = repos
        .codex_account
        .find_by_id(&account_id)
        .await
        .map_err(map_err)?;
    Ok(account.as_ref().map(CodexAccountDto::from))
}

#[tauri::command]
#[specta::specta]
pub async fn delete_codex_account(
    id: String,
    repos: State<'_, Repositories>,
) -> Result<(), CommandError> {
    let account_id = CodexAccountId::from_string(&id);
    repos
        .codex_account
        .delete(&account_id)
        .await
        .map_err(map_err)
}

// ─────────────────────── Quota Fetching ───────────────────────────────

#[tauri::command]
#[specta::specta]
pub async fn fetch_codex_quota(
    access_token: String,
    account_id: Option<String>,
) -> Result<CodexQuotaDto, CommandError> {
    fetch_codex_usage(&access_token, account_id.as_deref())
        .await
        .map(|quota| quota_to_dto(&quota))
        .map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub async fn refresh_codex_account_quota(
    id: String,
    repos: State<'_, Repositories>,
) -> Result<CodexAccountDto, CommandError> {
    let account_id_domain = CodexAccountId::from_string(&id);
    let mut account = repos
        .codex_account
        .find_by_id(&account_id_domain)
        .await
        .map_err(map_err)?
        .ok_or_else(|| "Account not found".to_string())?;

    let access_token = account
        .access_token()
        .ok_or("Account has no access token")?
        .to_string();
    let acct_id = account.account_id().map(|s| s.to_string());

    match fetch_codex_usage(&access_token, acct_id.as_deref()).await {
        Ok(quota) => {
            apply_usage_quota(&mut account, &quota);
            repos.codex_account.save(&account).await.map_err(map_err)?;
        }
        Err(e) => return Err(CommandError::infrastructure(format!("Failed to fetch quota: {}", e))),
    }

    Ok(CodexAccountDto::from(&account))
}

// ─────────────────────── Auth (active ~/.codex/auth.json) ─────────────

#[tauri::command]
#[specta::specta]
pub async fn get_active_codex_auth(
    repos: State<'_, Repositories>,
) -> Result<Option<CodexAuthInfoDto>, CommandError> {
    load_active_auth(&repos, false).await
}

#[tauri::command]
#[specta::specta]
pub async fn refresh_active_codex_auth_quota(
    repos: State<'_, Repositories>,
) -> Result<Option<CodexAuthInfoDto>, CommandError> {
    load_active_auth(&repos, true).await
}

#[tauri::command]
#[specta::specta]
pub async fn get_codex_account_inbox_code(
    id: String,
    repos: State<'_, Repositories>,
) -> Result<CodexInboxCodeDto, CommandError> {
    let account_id = CodexAccountId::from_string(&id);
    let account = repos
        .codex_account
        .find_by_id(&account_id)
        .await
        .map_err(map_err)?
        .ok_or_else(|| CommandError::not_found("Codex account not found"))?;

    let token = account
        .tempmail_token()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| CommandError::infrastructure("This account has no inbox token".to_string()))?;

    let client = TempmailClient::new(None)
        .map_err(|error| CommandError::infrastructure(error.to_string()))?;

    let code = client
        .poll_for_otp(token, None, Some(12), || false, |_| {})
        .await
        .map_err(|error| CommandError::infrastructure(format!("Failed to fetch inbox code: {}", error)))?
        .ok_or_else(|| CommandError::infrastructure("No verification code received".to_string()))?;

    Ok(CodexInboxCodeDto {
        email: account.email().to_string(),
        code,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn switch_codex_auth(
    account_id: String,
    repos: State<'_, Repositories>,
) -> Result<(), CommandError> {
    let acct_id = CodexAccountId::from_string(&account_id);
    let account = repos
        .codex_account
        .find_by_id(&acct_id)
        .await
        .map_err(map_err)?
        .ok_or("Account not found")?;

    let access_token = account
        .access_token()
        .ok_or("Account has no access token")?
        .to_string();
    let refresh_token = account
        .refresh_token()
        .ok_or("Account has no refresh token")?
        .to_string();
    let id_token = account
        .id_token()
        .unwrap_or("")
        .to_string();
    let openai_account_id = account
        .account_id()
        .unwrap_or("")
        .to_string();

    let auth = CodexAuthJson::chatgpt(id_token, access_token, refresh_token, openai_account_id);
    CodexAuthFile::write(&auth).map_err(map_err)?;

    info!("Switched Codex auth to account: {}", account.email());
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn set_codex_api_key(key: String) -> Result<(), CommandError> {
    let auth = CodexAuthJson::api_key(key);
    CodexAuthFile::write(&auth).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub async fn logout_codex_auth() -> Result<(), CommandError> {
    CodexAuthFile::clear().map_err(map_err)
}
