/// Codex account registration commands
///
/// Handles single and batch OpenAI account registration via Tempmail.lol.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::State;
use tracing::{info, warn};
use once_cell::sync::Lazy;

use crate::application::dtos::{RegisterConfig, RegisterMode, RegisterProgressDto, RegisterTaskStatus};
use crate::presentation::error::CommandError;
use crate::presentation::events::CodexRegisterProgress;
use crate::presentation::state::{Repositories, Services};
use super::quota::apply_usage_quota;
use neuradock_domain::codex::{CodexAccount, CodexAccountSource};
use neuradock_infrastructure::http::openai::quota::fetch_codex_usage;
use neuradock_infrastructure::http::openai::tempmail::TempmailClient;
use neuradock_infrastructure::http::openai::registrar::run_full_registration;

fn map_err(e: impl std::fmt::Display) -> CommandError {
    CommandError::from(e.to_string())
}

/// Global cancellation flag for active registration tasks
static CANCEL_FLAG: Lazy<Arc<AtomicBool>> =
    Lazy::new(|| Arc::new(AtomicBool::new(false)));

#[tauri::command]
#[specta::specta]
pub async fn cancel_codex_registration() -> Result<(), CommandError> {
    CANCEL_FLAG.store(true, Ordering::SeqCst);
    info!("Codex registration cancellation requested");
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn register_codex_accounts(
    config: RegisterConfig,
    app_handle: tauri::AppHandle,
    repos: State<'_, Repositories>,
    services: State<'_, Services>,
) -> Result<(), CommandError> {
    // Reset cancel flag
    CANCEL_FLAG.store(false, Ordering::SeqCst);

    let count = if config.mode == RegisterMode::Batch {
        config.count.unwrap_or(1).max(1)
    } else {
        1
    };
    let concurrency = config.concurrency.unwrap_or(1).clamp(1, 5);
    let min_interval = config.min_interval_secs.unwrap_or(3);
    let max_interval = config.max_interval_secs.unwrap_or(10);

    info!(
        "Starting Codex registration: count={}, concurrency={}, interval={}s-{}s",
        count, concurrency, min_interval, max_interval
    );

    let success_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let fail_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let completed = Arc::new(std::sync::atomic::AtomicU32::new(0));

    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency as usize));
    let repo = repos.codex_account.clone();
    let system_proxy_url = services
        .proxy_config
        .get()
        .await
        .map_err(CommandError::from)?
        .proxy_url();

    let mut tasks = vec![];

    for i in 0..count {
        if CANCEL_FLAG.load(Ordering::SeqCst) {
            break;
        }

        let permit = semaphore.clone().acquire_owned().await.map_err(map_err)?;
        let app = app_handle.clone();
        let repo = repo.clone();
        let proxy_url = system_proxy_url.clone();
        let cancel_flag = CANCEL_FLAG.clone();
        let success_count = success_count.clone();
        let fail_count = fail_count.clone();
        let completed = completed.clone();

        let task_id = format!("task-{}", i + 1);
        let total = count;

        let _ = emit_progress(
            &app_handle,
            RegisterProgressDto {
                task_id: task_id.clone(),
                email: None,
                status: RegisterTaskStatus::Running,
                message: format!("[任务 {}/{}] 初始化...", i + 1, count),
                current: completed.load(Ordering::SeqCst),
                total,
                success_count: success_count.load(Ordering::SeqCst),
                fail_count: fail_count.load(Ordering::SeqCst),
            },
        );

        if i > 0 {
            let sleep_secs = if min_interval >= max_interval {
                min_interval
            } else {
                use rand::Rng;
                rand::thread_rng().gen_range(min_interval..=max_interval)
            };
            tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;
        }

        let task = tokio::spawn(async move {
            let _permit = permit;

            if cancel_flag.load(Ordering::SeqCst) {
                return;
            }

            // Shared email once known (set after inbox creation)
            let current_email: std::sync::Arc<std::sync::Mutex<Option<String>>> =
                std::sync::Arc::new(std::sync::Mutex::new(None));
            let ce = std::sync::Arc::clone(&current_email);

            let result = register_single_account(
                &task_id,
                proxy_url.as_deref(),
                cancel_flag.clone(),
                |msg, email_hint| {
                    if let Some(ref e) = email_hint {
                        if let Ok(mut guard) = ce.lock() {
                            *guard = Some(e.clone());
                        }
                    }
                    let resolved_email = ce.lock().ok().and_then(|g| g.clone());
                    let _ = emit_progress(
                        &app,
                        RegisterProgressDto {
                            task_id: task_id.clone(),
                            email: resolved_email,
                            status: RegisterTaskStatus::Running,
                            message: msg,
                            current: completed.load(Ordering::SeqCst),
                            total,
                            success_count: success_count.load(Ordering::SeqCst),
                            fail_count: fail_count.load(Ordering::SeqCst),
                        },
                    );
                },
            )
            .await;

            let done = completed.fetch_add(1, Ordering::SeqCst) + 1;
            let resolved_email = current_email.lock().ok().and_then(|g| g.clone());

            match result {
                Ok(account) => {
                    let s = success_count.fetch_add(1, Ordering::SeqCst) + 1;
                    let f = fail_count.load(Ordering::SeqCst);
                    let email = account.email().to_string();

                    if let Err(e) = repo.save(&account).await {
                        warn!("Failed to save codex account {}: {}", email, e);
                    }

                    let _ = emit_progress(
                        &app,
                        RegisterProgressDto {
                            task_id: task_id.clone(),
                            email: Some(email.clone()),
                            status: RegisterTaskStatus::Success,
                            message: format!("[✅ {}/{}] {} 注册成功", done, total, email),
                            current: done,
                            total,
                            success_count: s,
                            fail_count: f,
                        },
                    );
                }
                Err(e) => {
                    let err_text = e.to_string();
                    if cancel_flag.load(Ordering::SeqCst) && err_text.contains("注册已取消") {
                        let s = success_count.load(Ordering::SeqCst);
                        let f = fail_count.load(Ordering::SeqCst);
                        let _ = emit_progress(
                            &app,
                            RegisterProgressDto {
                                task_id: task_id.clone(),
                                email: resolved_email,
                                status: RegisterTaskStatus::Cancelled,
                                message: format!("[⛔ {}/{}] 注册已取消", done, total),
                                current: done,
                                total,
                                success_count: s,
                                fail_count: f,
                            },
                        );
                    } else {
                        let f = fail_count.fetch_add(1, Ordering::SeqCst) + 1;
                        let s = success_count.load(Ordering::SeqCst);
                        warn!("Codex registration task {} failed: {:?}", task_id, e);

                        let _ = emit_progress(
                            &app,
                            RegisterProgressDto {
                                task_id: task_id.clone(),
                                email: resolved_email,
                                status: RegisterTaskStatus::Failed,
                                message: format!("[❌ {}/{}] 注册失败: {}", done, total, err_text),
                                current: done,
                                total,
                                success_count: s,
                                fail_count: f,
                            },
                        );
                    }
                }
            }
        });

        tasks.push(task);
    }

    let mut join_error: Option<String> = None;
    for t in tasks {
        if let Err(e) = t.await {
            warn!("Codex registration worker panicked or was aborted: {}", e);
            if join_error.is_none() {
                join_error = Some(e.to_string());
            }
        }
    }

    if let Some(err) = join_error {
        return Err(CommandError::infrastructure(format!(
            "注册任务异常中断: {}",
            err
        )));
    }

    info!(
        "Codex registration complete: success={}, fail={}",
        success_count.load(Ordering::SeqCst),
        fail_count.load(Ordering::SeqCst),
    );

    Ok(())
}

/// Full OpenAI account registration for a single account.
///
/// Flow (14 steps, all pure HTTP via reqwest):
/// 1. Create Tempmail inbox
/// 2-13. Full OAuth PKCE flow (device ID, sentinel, signup, password, OTP, tokens)
async fn register_single_account(
    task_id: &str,
    proxy_url: Option<&str>,
    cancel_flag: Arc<AtomicBool>,
    mut progress: impl FnMut(String, Option<String>),
) -> anyhow::Result<CodexAccount> {
    let mut log = |msg: &str, email_hint: Option<String>| {
        progress(format!("[{}] {}", task_id, msg), email_hint);
    };

    let is_cancelled = || cancel_flag.load(Ordering::SeqCst);
    if is_cancelled() {
        return Err(anyhow::anyhow!("注册已取消"));
    }

    // ── 1. Create Tempmail inbox ─────────────────────────────────────────────
    log("📬 创建临时邮箱...", None);
    let tempmail = TempmailClient::new(proxy_url)?;
    let inbox = tempmail.create_inbox().await?;

    // Emit email once known so console header updates immediately
    log(
        &format!("📧 邮箱: {}", inbox.address),
        Some(inbox.address.clone()),
    );

    let inbox_token = inbox.token.clone();
    let inbox_address = inbox.address.clone();

    // Clone for the closure
    let proxy_owned = proxy_url.map(|s| s.to_string());

    // ── 2+. OAuth registration + login + token exchange ──────────────────────
    let result = run_full_registration(
        &inbox_address,
        proxy_owned.as_deref(),
        |msg| log(msg, None),
        |otp_sent_at| {
            let token = inbox_token.clone();
            let proxy = proxy_owned.clone();
            let cancel_flag = cancel_flag.clone();
            async move {
                if cancel_flag.load(Ordering::SeqCst) {
                    return Err(anyhow::anyhow!("注册已取消"));
                }
                let tm = TempmailClient::new(proxy.as_deref())?;
                let otp = tm
                    .poll_for_otp(&token, Some(otp_sent_at), Some(120), || {
                        cancel_flag.load(Ordering::SeqCst)
                    }, |msg| {
                        tracing::debug!("[tempmail poll] {}", msg);
                    })
                    .await?;
                otp.ok_or_else(|| anyhow::anyhow!("等待验证码超时 (120s)"))
            }
        },
    )
    .await?;

    // ── Build domain aggregate ────────────────────────────────────────────────
    let mut account = CodexAccount::new(
        result.email.clone(),
        Some(result.password.clone()),
        CodexAccountSource::Register,
    )
    .map_err(|e| anyhow::anyhow!("{}", e))?;

    account.set_tempmail_token(inbox.token);

    // Parse expires_at from RFC-3339 string to DateTime<Utc>
    let expires_dt = chrono::DateTime::parse_from_rfc3339(&result.expires_at)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc));

    account.apply_tokens(
        result.access_token,
        result.refresh_token,
        result.id_token,
        if result.account_id.is_empty() { None } else { Some(result.account_id) },
        expires_dt,
    );

    if let Some(cookie_header) = result
        .web_session_cookie
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        account.set_payment_session(
            cookie_header.to_string(),
            result
                .web_session_device_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
        );
    }

    if let Some(access_token) = account.access_token().map(|token| token.to_string()) {
        log("💳 同步额度信息...", None);
        let account_id = account.account_id().map(|id| id.to_string());
        match fetch_codex_usage(&access_token, account_id.as_deref()).await {
            Ok(quota) => {
                apply_usage_quota(&mut account, &quota);
                let weekly_remaining = quota
                    .secondary_window
                    .as_ref()
                    .map(|window| (100.0 - window.used_percent).clamp(0.0, 100.0))
                    .map(|value| format!("{value:.0}%"))
                    .unwrap_or_else(|| "未知".to_string());
                let reset_at = quota
                    .secondary_window
                    .as_ref()
                    .and_then(|window| window.resets_at)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
                    .unwrap_or_else(|| "未知".to_string());
                log(&format!("  周额度: {}，重置 {}", weekly_remaining, reset_at), None);
            }
            Err(e) => {
                warn!("Failed to sync Codex quota during registration: {}", e);
                log(&format!("⚠️ 额度同步失败: {}", e), None);
            }
        }
    }

    Ok(account)
}

fn emit_progress(app: &tauri::AppHandle, payload: RegisterProgressDto) -> Result<(), String> {
    use tauri::Emitter;
    let event = CodexRegisterProgress {
        task_id: payload.task_id,
        email: payload.email,
        status: payload.status,
        message: payload.message,
        current: payload.current,
        total: payload.total,
        success_count: payload.success_count,
        fail_count: payload.fail_count,
    };

    app.emit("codex-register-progress", &event)
        .map_err(|e| e.to_string())
}
