use anyhow::Result;
use chrono::Utc;
use log::{error, info};
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;

use neuradock_domain::notification::{
    NotificationChannelRepository, NotificationMessage, NotificationSender,
};
use neuradock_infrastructure::notification::create_sender;
use crate::application::services::i18n::t;

#[derive(FromRow)]
struct YesterdayBalanceRow {
    current_balance: f64,
    total_consumed: f64,
    total_income: f64,
}

/// Notification application service
/// Coordinates sending notifications through enabled channels
pub struct NotificationService {
    channel_repo: Arc<dyn NotificationChannelRepository>,
    db: Arc<SqlitePool>,
}

impl NotificationService {
    pub fn new(channel_repo: Arc<dyn NotificationChannelRepository>, db: Arc<SqlitePool>) -> Self {
        Self { channel_repo, db }
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
        let yesterday = (Utc::now() - chrono::Duration::days(1)).date_naive();
        let yesterday_str = yesterday.format("%Y-%m-%d").to_string();

        let query = r#"
            SELECT
                current_balance,
                total_consumed,
                total_income
            FROM balance_history
            WHERE account_id = ?1 AND DATE(recorded_at) = ?2
            ORDER BY recorded_at DESC
            LIMIT 1
        "#;

        match sqlx::query_as::<_, YesterdayBalanceRow>(query)
            .bind(account_id)
            .bind(&yesterday_str)
            .fetch_optional(&*self.db)
            .await
        {
            Ok(Some(row)) => {
                info!("Found yesterday's balance for account {}: current={}, consumed={}, income={}",
                    account_id, row.current_balance, row.total_consumed, row.total_income);
                Some((row.current_balance, row.total_consumed, row.total_income))
            }
            Ok(None) => {
                info!("No yesterday balance found for account {}", account_id);
                None
            }
            Err(e) => {
                error!("Failed to query yesterday's balance: {}", e);
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
        balance: Option<(f64, f64, f64)>, // (current_balance, total_consumed, total_income)
    ) -> Result<()> {
        let yesterday_balance = self.get_yesterday_balance(account_id).await;

        let content = if let Some((today_current, today_consumed, today_income)) = balance {
            if let Some((yesterday_current, yesterday_consumed, yesterday_income)) = yesterday_balance {
                // Calculate changes
                let current_change = today_current - yesterday_current;
                let consumed_change = today_consumed - yesterday_consumed;
                let income_change = today_income - yesterday_income;

                let current_emoji = if current_change > 0.0 { "📈" } else if current_change < 0.0 { "📉" } else { "➡️" };
                let consumed_emoji = if consumed_change > 0.0 { "📈" } else if consumed_change < 0.0 { "📉" } else { "➡️" };
                let income_emoji = if income_change > 0.0 { "📈" } else if income_change < 0.0 { "📉" } else { "➡️" };

                format!(
                    "{}: {}\n{}: {}\n\n{}:\n   {}: ${:.2}\n   {}: ${:.2}\n   {}: ${:.2}\n\n{}:\n   {}: ${:.2} {}\n   {}: ${:.2} {}\n   {}: ${:.2} {}\n\n{}:\n   {}: {:+.2} {}\n   {}: {:+.2} {}\n   {}: {:+.2} {}",
                    t("notification.label.account"),
                    account_name,
                    t("notification.label.provider"),
                    provider_name,
                    t("notification.label.yesterday"),
                    t("notification.label.currentBalance"),
                    yesterday_current,
                    t("notification.label.totalConsumed"),
                    yesterday_consumed,
                    t("notification.label.totalIncome"),
                    yesterday_income,
                    t("notification.label.today"),
                    t("notification.label.currentBalance"),
                    today_current,
                    current_emoji,
                    t("notification.label.totalConsumed"),
                    today_consumed,
                    consumed_emoji,
                    t("notification.label.totalIncome"),
                    today_income,
                    income_emoji,
                    t("notification.label.changes"),
                    t("notification.label.currentBalance"),
                    current_change,
                    "$",
                    t("notification.label.totalConsumed"),
                    consumed_change,
                    "$",
                    t("notification.label.totalIncome"),
                    income_change,
                    "$"
                )
            } else {
                // No yesterday data, just show today
                format!(
                    "{}: {}\n{}: {}\n\n{}:\n   {}: ${:.2}\n   {}: ${:.2}\n   {}: ${:.2}",
                    t("notification.label.account"),
                    account_name,
                    t("notification.label.provider"),
                    provider_name,
                    t("notification.label.today"),
                    t("notification.label.currentBalance"),
                    today_current,
                    t("notification.label.totalConsumed"),
                    today_consumed,
                    t("notification.label.totalIncome"),
                    today_income
                )
            }
        } else {
            format!("{}: {}\n{}: {}\n\n{}",
                t("notification.label.account"), account_name,
                t("notification.label.provider"), provider_name,
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
        let content = format!(
            "{}: {}\n{}: {}\n\n❌ {}: {}",
            t("notification.label.account"),
            account_name,
            t("notification.label.provider"),
            provider_name,
            t("notification.label.error"),
            error
        );

        let message = NotificationMessage::new(t("notification.checkIn.failure.title"), content);

        self.send_to_all(&message).await
    }

    /// Test notification channel
    pub async fn test_channel(&self, sender: Arc<dyn NotificationSender>) -> Result<()> {
        sender.test().await.map_err(|e| anyhow::anyhow!(e.to_string()))
    }
}
