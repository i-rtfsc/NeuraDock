use chrono::{Local, Timelike};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use neuradock_domain::account::AccountRepository;
use neuradock_domain::balance_history::BalanceHistoryRepository;
use neuradock_domain::notification::{NotificationChannelRepository, NotificationMessage};
use neuradock_domain::reminder_settings::{
    ReminderChannel, ReminderSettings, ReminderSettingsRepository,
};
use neuradock_infrastructure::notification::create_sender;

/// Independent reminder loop separate from the check-in scheduler.
///
/// The scheduler *executes* check-ins at a fixed wall-clock time per account;
/// this loop *inspects the result* and alerts when one is missed. Keeping
/// them decoupled means a reminder failure never interferes with the actual
/// check-in attempt, and vice versa.
pub struct ReminderLoop {
    account_repo: Arc<dyn AccountRepository>,
    history_repo: Arc<dyn BalanceHistoryRepository>,
    settings_repo: Arc<dyn ReminderSettingsRepository>,
    /// Reused so reminders go to the same Feishu webhooks the user already
    /// configured for check-in results, rather than a second place to set up.
    channel_repo: Arc<dyn NotificationChannelRepository>,
    app_handle: tauri::AppHandle,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl ReminderLoop {
    pub fn new(
        account_repo: Arc<dyn AccountRepository>,
        history_repo: Arc<dyn BalanceHistoryRepository>,
        settings_repo: Arc<dyn ReminderSettingsRepository>,
        channel_repo: Arc<dyn NotificationChannelRepository>,
        app_handle: tauri::AppHandle,
    ) -> Self {
        Self {
            account_repo,
            history_repo,
            settings_repo,
            channel_repo,
            app_handle,
            handle: Mutex::new(None),
        }
    }

    /// Launch a background task that wakes every ~30 s, reads the
    /// settings, and fires reminders at the configured times.
    pub async fn start(&self) {
        let mut guard = self.handle.lock().await;
        if guard.is_some() {
            return;
        }

        info!("🔔 Reminder loop started (every 30s, watches settings)");

        let repo = self.account_repo.clone();
        let history = self.history_repo.clone();
        let settings_repo = self.settings_repo.clone();
        let channel_repo = self.channel_repo.clone();
        let app = self.app_handle.clone();

        let handle = tokio::spawn(async move {
            // Owned by the task. Wakeups are more frequent than the one-minute
            // resolution of a time point, so this records the minute we last
            // acted on and suppresses repeats within it.
            let mut last_tick: Option<chrono::NaiveDateTime> = None;
            // Remembers the day we already announced as complete, so the
            // "all done" notice fires once rather than on every wakeup.
            let mut announced_done_on: Option<chrono::NaiveDate> = None;

            loop {
                let now = Local::now();
                let today = now.date_naive();
                let this_minute = today.and_hms_opt(now.hour(), now.minute(), 0);

                let is_new_minute = match (this_minute, last_tick) {
                    (Some(current), Some(previous)) => current > previous,
                    (Some(_), None) => true,
                    (None, _) => false,
                };

                if is_new_minute {
                    last_tick = this_minute;

                    match settings_repo.get().await {
                        Ok(settings) => {
                            if settings.is_due_at(now.hour(), now.minute()) {
                                // Honour "stop when all done" before sending:
                                // a check-in completed between two time points
                                // should cancel the later one.
                                info!("⏰ Reminder due at {:02}:{:02}", now.hour(), now.minute());

                                if let Err(e) = run_reminder_tick(
                                    &*repo,
                                    &*history,
                                    &*channel_repo,
                                    &settings,
                                    &app,
                                    today,
                                )
                                .await
                                {
                                    error!("Reminder tick failed: {e}");
                                }
                            }

                            if settings.enabled()
                                && settings.stop_when_all_done()
                                && announced_done_on != Some(today)
                            {
                                match all_accounts_done(&*repo, &*history, today).await {
                                    Ok(true) => {
                                        announced_done_on = Some(today);
                                        info!(
                                            "Reminder: all accounts checked in for {today}; \
                                             remaining reminders suppressed"
                                        );
                                    }
                                    Ok(false) => {}
                                    Err(e) => {
                                        warn!("Reminder completion check failed: {e}");
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Reminder could not read settings: {e}");
                        }
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            }
        });

        *guard = Some(handle);
    }
}

/// Core logic: find undone check-ins and notify.
pub async fn run_reminder_tick(
    account_repo: &dyn AccountRepository,
    history_repo: &dyn BalanceHistoryRepository,
    channel_repo: &dyn NotificationChannelRepository,
    settings: &ReminderSettings,
    app: &tauri::AppHandle,
    date: chrono::NaiveDate,
) -> Result<(), Box<dyn std::error::Error>> {
    let enabled = account_repo.find_enabled().await?;
    if enabled.is_empty() {
        return Ok(());
    }

    // Figure out which enabled accounts haven't checked in today.
    let mut missed = Vec::new();
    for account in &enabled {
        let record = history_repo
            .find_latest_by_account_id_on_date(account.id(), date)
            .await?;
        if record.is_none() {
            // Fetch balance snapshot for the notification
            let balance: Option<String> = history_repo
                .find_latest_by_account_id(account.id())
                .await?
                .map(|r| format!("${:.2}", r.current_balance()));
            missed.push((
                account.name().to_string(),
                account.provider_id().as_str().to_string(),
                balance,
            ));
        }
    }

    if missed.is_empty() {
        return Ok(());
    }

    let date_str = date.format("%Y-%m-%d").to_string();
    let content = build_missed_check_in_message(&missed, &date_str);

    for channel in settings.channels() {
        match channel {
            ReminderChannel::App => {
                let title = format!("今日签到提醒 · {}", date_str);
                let body = build_missed_app_body(&missed);
                send_app_notification(app, &title, &body);
            }
            ReminderChannel::Feishu => {
                let message =
                    NotificationMessage::new(format!("🔔 签到提醒 · {date_str}"), content.clone());
                send_feishu_via_channels(channel_repo, &message).await;
            }
        }
    }

    info!(
        "Sent missed check-in reminder for {} account(s)",
        missed.len()
    );
    Ok(())
}

/// Whether every enabled account already has a check-in recorded for `date`.
async fn all_accounts_done(
    account_repo: &dyn AccountRepository,
    history_repo: &dyn BalanceHistoryRepository,
    date: chrono::NaiveDate,
) -> Result<bool, Box<dyn std::error::Error>> {
    let enabled = account_repo.find_enabled().await?;
    if enabled.is_empty() {
        return Ok(false);
    }

    for account in &enabled {
        if history_repo
            .find_latest_by_account_id_on_date(account.id(), date)
            .await?
            .is_none()
        {
            return Ok(false);
        }
    }

    Ok(true)
}

// ---------------------------------------------------------------------------
// Notification helpers
// ---------------------------------------------------------------------------

fn build_missed_check_in_message(
    missed: &[(String, String, Option<String>)],
    date: &str,
) -> String {
    let mut content = String::new();
    content.push_str(&format!("📅 日期: {date}\n\n"));
    content.push_str("以下账号今日尚未签到：\n\n");

    for (idx, (name, provider, balance)) in missed.iter().enumerate() {
        content.push_str(&format!("{}. **{}** ({})\n", idx + 1, name, provider));
        if let Some(bal) = balance {
            content.push_str(&format!("   当前余额: {bal}\n"));
        }
        content.push('\n');
    }

    content.push_str(&format!("---\n共 {} 个账号待签到", missed.len()));
    content
}

fn build_missed_app_body(missed: &[(String, String, Option<String>)]) -> String {
    let names: Vec<_> = missed.iter().map(|(name, _, _)| name.as_str()).collect();
    if names.len() <= 3 {
        format!("今日未签到: {}", names.join("、"))
    } else {
        format!("今日未签到: {} 等 {} 个账号", names[0], names.len())
    }
}

fn send_app_notification(app: &tauri::AppHandle, title: &str, body: &str) {
    use tauri_plugin_notification::NotificationExt;

    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .unwrap_or_else(|e| {
            warn!("Failed to show app notification: {e}");
        });
}

async fn send_feishu_via_channels(
    channel_repo: &dyn NotificationChannelRepository,
    message: &NotificationMessage,
) {
    let channels = match channel_repo.find_all_enabled().await {
        Ok(channels) => channels,
        Err(e) => {
            error!("Reminder could not load notification channels: {e}");
            return;
        }
    };

    if channels.is_empty() {
        warn!("Reminder wanted to notify via Feishu but no channel is enabled");
        return;
    }

    for channel in channels {
        let sender = match create_sender(channel.config()) {
            Ok(sender) => sender,
            // DingTalk and Email are still stubs; skipping them here keeps a
            // configured-but-unimplemented channel from masking the Feishu one.
            Err(e) => {
                warn!(
                    "Reminder skipped channel {} ({}): {e}",
                    channel.id(),
                    channel.channel_type()
                );
                continue;
            }
        };

        if let Err(e) = sender.send(message).await {
            error!("Failed to send reminder via channel {}: {e}", channel.id());
        }
    }
}
