use log::{error, info};
use std::sync::Arc;

use crate::application::dtos::BalanceDto;
use crate::application::services::{NotificationService, ProviderModelsService};
use neuradock_domain::balance_history::CheckInOutcome;
use neuradock_domain::{
    account::{Account, AccountRepository},
    check_in::Provider,
    shared::{AccountId, DomainError},
};
use neuradock_infrastructure::http::UserInfo;

/// Refresh the account's balance cache after a check-in.
///
/// The balance *history* snapshot is written by the executor, which every
/// caller goes through, so this only maintains the account row. The check-in
/// timestamps are not touched: they are a cache of the ledger.
pub async fn update_and_save_balance(
    account_repo: &Arc<dyn AccountRepository>,
    account_id: &str,
    mut account: Account,
    user_info: &UserInfo,
) -> Result<BalanceDto, DomainError> {
    // Compensate for quota the provider lost from its own books before the
    // value is stored, so the correction lands in the account row too.
    let adjusted_total_quota = account.adjusted_total_quota(user_info.total_quota);

    account.update_balance(
        user_info.current_balance,
        user_info.total_consumed,
        adjusted_total_quota,
    );

    if let Err(e) = account_repo.save(&account).await {
        error!("Failed to save account balance after check-in: {}", e);
    } else {
        info!(
            "Account {} balance updated in database: current=${:.2}, consumed=${:.2}, total_quota=${:.2}",
            account_id, user_info.current_balance, user_info.total_consumed, adjusted_total_quota
        );
    }

    Ok(BalanceDto {
        current_balance: user_info.current_balance,
        total_consumed: user_info.total_consumed,
        total_quota: adjusted_total_quota,
    })
}

/// Auto-fetch provider models if not exists in database
pub async fn auto_fetch_provider_models(
    account_repo: &Arc<dyn AccountRepository>,
    provider_models_service: &Arc<ProviderModelsService>,
    account_id: &str,
    provider: &Provider,
) {
    if provider_models_service
        .should_auto_fetch(provider.id().as_str())
        .await
    {
        info!("No provider models in database, auto-fetching...");
        // Reload account to get the latest cookies (may have been updated during check-in)
        if let Ok(Some(updated_acc)) = account_repo
            .find_by_id(&AccountId::from_string(account_id))
            .await
        {
            provider_models_service
                .fetch_and_save_provider_models(
                    provider,
                    updated_acc.credentials().cookies(),
                    updated_acc.credentials().api_user(),
                )
                .await;
        }
    } else {
        info!("Provider models already exist in database, skipping auto-fetch");
    }
}

/// Send a check-in notification, when there is news to send.
///
/// A day that was already settled and a run the guard skipped are not news, so
/// they stay silent; only a real attempt reports success or failure.
pub async fn send_check_in_notification(
    notification_service: &Option<Arc<NotificationService>>,
    outcome: CheckInOutcome,
    account_id: &str,
    account_name: &str,
    provider_name: &str,
    message: &str,
    balance: Option<(f64, f64, f64)>, // (current_balance, total_consumed, total_quota)
) {
    if matches!(
        outcome,
        CheckInOutcome::AlreadyConfirmed | CheckInOutcome::SkippedGuard | CheckInOutcome::Cancelled
    ) {
        info!(
            "Skipping the check-in notification for {} ({:?})",
            account_id, outcome
        );
        return;
    }

    if let Some(notification_service) = notification_service {
        if outcome == CheckInOutcome::Success {
            // Send success notification
            if let Err(e) = notification_service
                .send_check_in_success(account_id, account_name, provider_name, balance)
                .await
            {
                error!("Failed to send check-in success notification: {}", e);
            } else {
                info!(
                    "Check-in success notification sent for account {}",
                    account_id
                );
            }
        } else {
            // Send failure notification
            if let Err(e) = notification_service
                .send_check_in_failure(account_name, provider_name, message)
                .await
            {
                error!("Failed to send check-in failure notification: {}", e);
            } else {
                info!(
                    "Check-in failure notification sent for account {}",
                    account_id
                );
            }
        }
    } else {
        info!(
            "Notification service not available for account {}",
            account_id
        );
    }
}
