use std::sync::Arc;

use neuradock_domain::account::AccountRepository;
use neuradock_domain::check_in::ProviderRepository;
use neuradock_domain::proxy_config::ProxyConfigRepository;
use neuradock_domain::shared::{AccountId, DomainError};

use crate::application::dtos::BalanceDto;
use crate::application::services::{BalanceHistoryService, CheckInExecutor};

pub struct BalanceService {
    account_repo: Arc<dyn AccountRepository>,
    provider_repo: Arc<dyn ProviderRepository>,
    balance_history_service: Arc<BalanceHistoryService>,
    proxy_config_repo: Arc<dyn ProxyConfigRepository>,
    headless_browser: bool,
}

impl BalanceService {
    pub fn new(
        account_repo: Arc<dyn AccountRepository>,
        provider_repo: Arc<dyn ProviderRepository>,
        balance_history_service: Arc<BalanceHistoryService>,
        proxy_config_repo: Arc<dyn ProxyConfigRepository>,
        headless_browser: bool,
    ) -> Self {
        Self {
            account_repo,
            provider_repo,
            balance_history_service,
            proxy_config_repo,
            headless_browser,
        }
    }

    pub async fn fetch_account_balance(
        &self,
        account_id: &str,
        force_refresh: bool,
    ) -> Result<BalanceDto, DomainError> {
        const MAX_CACHE_AGE_HOURS: i64 = 1;

        let acc_id = AccountId::from_string(account_id);
        let account = self
            .account_repo
            .find_by_id(&acc_id)
            .await?
            .ok_or_else(|| DomainError::AccountNotFound(account_id.to_string()))?;

        if !force_refresh && !account.is_balance_stale(MAX_CACHE_AGE_HOURS) {
            if let (Some(current_balance), Some(total_consumed), Some(total_quota)) = (
                account.current_balance(),
                account.total_consumed(),
                account.total_quota(),
            ) {
                return Ok(BalanceDto {
                    current_balance,
                    total_consumed,
                    total_quota,
                });
            }
        }

        let provider = self
            .provider_repo
            .find_by_id(account.provider_id())
            .await?
            .ok_or_else(|| {
                DomainError::ProviderNotFound(account.provider_id().as_str().to_string())
            })?;

        let proxy_url = self.proxy_config_repo.get().await?.proxy_url();
        let executor = CheckInExecutor::with_proxy(
            self.account_repo.clone(),
            self.headless_browser,
            proxy_url,
        )
        .map_err(|e| DomainError::Infrastructure(e.to_string()))?;
        let user_info = executor
            .fetch_balance_only(account_id, &provider)
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;

        // Re-load account since fetch_balance_only may have updated cookies
        let mut account = self
            .account_repo
            .find_by_id(&acc_id)
            .await?
            .ok_or_else(|| DomainError::AccountNotFound(account_id.to_string()))?;

        let current_balance = user_info.current_balance;
        let total_consumed = user_info.total_consumed;
        let balance_dto = BalanceDto {
            current_balance,
            total_consumed,
            total_quota: current_balance + total_consumed,
        };

        account.update_balance(
            balance_dto.current_balance,
            balance_dto.total_consumed,
            balance_dto.total_quota,
        );
        self.account_repo.save(&account).await?;

        let _ = self
            .balance_history_service
            .save_balance_history(account_id, &balance_dto)
            .await;

        Ok(balance_dto)
    }
}
