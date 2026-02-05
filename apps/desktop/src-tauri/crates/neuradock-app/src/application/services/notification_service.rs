use anyhow::Result;
use chrono::Utc;
use log::{error, info};
use std::sync::Arc;

use crate::application::services::i18n::t;
use neuradock_domain::balance_history::{BalanceHistoryRecord, BalanceHistoryRepository};
use neuradock_domain::notification::{NotificationChannelRepository, NotificationMessage};
use neuradock_domain::shared::AccountId;
use neuradock_infrastructure::notification::create_sender;

/// Notification application service
/// Coordinates sending notifications through enabled channels
pub struct NotificationService {
    channel_repo: Arc<dyn NotificationChannelRepository>,
    balance_history_repo: Arc<dyn BalanceHistoryRepository>,
}

impl NotificationService {
    pub fn new(
        channel_repo: Arc<dyn NotificationChannelRepository>,
        balance_history_repo: Arc<dyn BalanceHistoryRepository>,
    ) -> Self {
        Self {
            channel_repo,
            balance_history_repo,
        }
    }

    /// Send notification to all enabled channels
    pub async fn send_to_all(&self, message: &NotificationMessage) -> Result<()> {
        let channels = self.channel_repo.find_all_enabled().await?;

        if channels.is_empty() {
            info!("No enabled notification channels configured, skipping notification");
            return Ok(());
        }

        info!(
            "Sending notification to {} enabled channel(s): {}",
            channels.len(),
            message.title
        );

        for channel in channels {
            let sender = match create_sender(channel.config()) {
                Ok(s) => s,
                Err(e) => {
                    error!(
                        "Failed to create sender for channel {}: {}",
                        channel.id(),
                        e
                    );
                    continue;
                }
            };

            if let Err(e) = sender.send(message).await {
                error!(
                    "Failed to send notification via channel {} ({}): {}",
                    channel.id(),
                    channel.channel_type(),
                    e
                );
            } else {
                info!(
                    "Successfully sent notification via channel {} ({})",
                    channel.id(),
                    channel.channel_type()
                );
            }
        }

        Ok(())
    }

    /// Get yesterday's balance data from balance_history
    async fn get_yesterday_balance(&self, account_id: &str) -> Option<(f64, f64, f64)> {
        let today = Utc::now().date_naive();
        let yesterday = today - chrono::Duration::days(1);
        let account_id_obj = AccountId::from_string(account_id);

        let convert_record = |record: BalanceHistoryRecord| {
            Some((
                record.current_balance(),
                record.total_consumed(),
                record.total_quota(),
            ))
        };

        match self
            .balance_history_repo
            .find_latest_by_account_id_on_date(&account_id_obj, yesterday)
            .await
        {
            Ok(Some(record)) => {
                info!(
                    "Found yesterday's balance for account {}: current={}, consumed={}, quota={}",
                    account_id,
                    record.current_balance(),
                    record.total_consumed(),
                    record.total_quota()
                );
                return convert_record(record);
            }
            Ok(None) => {
                info!(
                    "No exact-dated yesterday balance found for account {}",
                    account_id
                );
            }
            Err(e) => {
                error!(
                    "Failed to query yesterday's balance for account {}: {}",
                    account_id, e
                );
            }
        }

        // Fallback: grab all daily summaries and find the latest one that is before today.
        match self
            .balance_history_repo
            .list_all_daily_summaries(&account_id_obj)
            .await
        {
            Ok(summaries) => {
                // Find the latest summary that is strictly before today
                if let Some(record) = summaries
                    .into_iter()
                    .filter(|s| s.check_in_date() < today)
                    .last()
                {
                    info!(
                        "Using fallback balance from summary for account {} from {}",
                        account_id, record.check_in_date()
                    );
                    return Some((
                        record.daily_balance(),
                        record.daily_consumed(),
                        record.daily_total_quota(),
                    ));
                }
                
                info!(
                    "No historical balance summaries available for account {} before today",
                    account_id
                );
                None
            }
            Err(e) => {
                error!(
                    "Failed to query balance summaries for account {}: {}",
                    account_id, e
                );
                None
            }
        }
    }

    /// Send check-in success notification with yesterday/today comparison
    pub async fn send_check_in_success(
        &self,
        account_id: &str,
        account_name: &str,
        provider_name: &str,
        balance: Option<(f64, f64, f64)>, // (current_balance, total_consumed, total_quota)
    ) -> Result<()> {
        let yesterday_balance = self.get_yesterday_balance(account_id).await;
        let now = chrono::Local::now();
        let time_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        let content = if let Some((today_current, today_consumed, today_income)) = balance {
            if let Some((yesterday_current, yesterday_consumed, yesterday_income)) =
                yesterday_balance
            {
                // Calculate changes
                let current_change = today_current - yesterday_current;
                let consumed_change = today_consumed - yesterday_consumed;
                let income_change = today_income - yesterday_income;

                let current_emoji = if current_change > 0.0 {
                    "📈"
                } else if current_change < 0.0 {
                    "📉"
                } else {
                    "➡️"
                };
                let consumed_emoji = if consumed_change > 0.0 {
                    "📈"
                } else if consumed_change < 0.0 {
                    "📉"
                } else {
                    "➡️"
                };
                let income_emoji = if income_change > 0.0 {
                    "📈"
                } else if income_change < 0.0 {
                    "📉"
                } else {
                    "➡️"
                };

                format!(
                    "{}: {}\n{}: {}\n{}: {}\n\n{}:\n   {}: ${:.2}\n   {}: ${:.2}\n   {}: ${:.2}\n\n{}:\n   {}: ${:.2} {}\n   {}: ${:.2} {}\n   {}: ${:.2} {}\n\n{}:\n   {}: {:+.2} {}\n   {}: {:+.2} {}\n   {}: {:+.2} {}",
                    t("notification.label.account"),
                    account_name,
                    t("notification.label.provider"),
                    provider_name,
                    t("notification.label.time"),
                    time_str,
                    t("notification.label.yesterday"),
                    t("notification.label.currentBalance"),
                    yesterday_current,
                    t("notification.label.totalConsumed"),
                    yesterday_consumed,
                    t("notification.label.totalQuota"),
                    yesterday_income,
                    t("notification.label.today"),
                    t("notification.label.currentBalance"),
                    today_current,
                    current_emoji,
                    t("notification.label.totalConsumed"),
                    today_consumed,
                    consumed_emoji,
                    t("notification.label.totalQuota"),
                    today_income,
                    income_emoji,
                    t("notification.label.changes"),
                    t("notification.label.currentBalance"),
                    current_change,
                    "$",
                    t("notification.label.totalConsumed"),
                    consumed_change,
                    "$",
                    t("notification.label.totalQuota"),
                    income_change,
                    "$"
                )
            } else {
                // No yesterday data, just show today
                format!(
                    "{}: {}\n{}: {}\n{}: {}\n\n{}:\n   {}: ${:.2}\n   {}: ${:.2}\n   {}: ${:.2}",
                    t("notification.label.account"),
                    account_name,
                    t("notification.label.provider"),
                    provider_name,
                    t("notification.label.time"),
                    time_str,
                    t("notification.label.today"),
                    t("notification.label.currentBalance"),
                    today_current,
                    t("notification.label.totalConsumed"),
                    today_consumed,
                    t("notification.label.totalQuota"),
                    today_income
                )
            }
        } else {
            format!(
                "{}: {}\n{}: {}\n{}: {}\n\n{}",
                t("notification.label.account"),
                account_name,
                t("notification.label.provider"),
                provider_name,
                t("notification.label.time"),
                time_str,
                t("notification.checkIn.success.simple")
            )
        };

        let message = NotificationMessage::new(t("notification.checkIn.success.title"), content);

        self.send_to_all(&message).await
    }

    /// Send check-in failure notification
    pub async fn send_check_in_failure(
        &self,
        account_name: &str,
        provider_name: &str,
        error: &str,
    ) -> Result<()> {
        let now = chrono::Local::now();
        let time_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        let content = format!(
            "{}: {}\n{}: {}\n{}: {}\n\n❌ {}: {}",
            t("notification.label.account"),
            account_name,
            t("notification.label.provider"),
            provider_name,
            t("notification.label.time"),
            time_str,
            t("notification.label.error"),
            error
        );

        let message = NotificationMessage::new(t("notification.checkIn.failure.title"), content);

        self.send_to_all(&message).await
    }
}
