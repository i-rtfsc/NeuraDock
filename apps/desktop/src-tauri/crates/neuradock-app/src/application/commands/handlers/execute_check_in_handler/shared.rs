use log::{error, info};
use std::sync::Arc;

use crate::application::dtos::BalanceDto;
use crate::application::services::{BalanceHistoryService, ProviderModelsService, NotificationService};
use neuradock_domain::{
    account::{Account, AccountRepository},
    check_in::Provider,
    shared::{AccountId, DomainError},
};
use neuradock_infrastructure::http::UserInfo;

/// Update account balance cache and save to balance_history
pub async fn update_and_save_balance(
    account_repo: &Arc<dyn AccountRepository>,
    balance_history_service: &Arc<BalanceHistoryService>,
    account_id: &str,
    mut account: Account,
    user_info: &UserInfo,
) -> Result<BalanceDto, DomainError> {
    account.update_balance(
        user_info.quota,
        user_info.used_quota,
        user_info.quota + user_info.used_quota,
    );

    // Save updated account with new balance
    if let Err(e) = account_repo.save(&account).await {
        error!("Failed to save account balance after check-in: {}", e);
    } else {
        info!(
            "Account {} balance updated in database: current=${:.2}, consumed=${:.2}, income=${:.2}",
            account_id, user_info.quota, user_info.used_quota, user_info.quota + user_info.used_quota
        );
    }

    // Build balance DTO
    let balance = BalanceDto {
        current_balance: user_info.quota,
        total_consumed: user_info.used_quota,
        total_income: user_info.quota + user_info.used_quota,
    };

    // Save to balance_history table
    balance_history_service
        .save_balance_history(account_id, &balance)
        .await;

    Ok(balance)
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

/// Send check-in notification (success or failure)
pub async fn send_check_in_notification(
    notification_service: &Option<Arc<NotificationService>>,
    success: bool,
    account_id: &str,
    account_name: &str,
    provider_name: &str,
    message: &str,
    balance: Option<(f64, f64, f64)>, // (quota, used_quota, total_income)
) {
    if let Some(notification_service) = notification_service {
        if success {
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
