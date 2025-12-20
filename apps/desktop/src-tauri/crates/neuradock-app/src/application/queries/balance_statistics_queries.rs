use std::collections::HashMap;
use std::sync::Arc;

use neuradock_domain::account::AccountRepository;
use neuradock_domain::check_in::ProviderRepository;
use neuradock_domain::shared::DomainError;

use crate::application::dtos::{BalanceStatisticsDto, ProviderBalanceDto};
use crate::application::services::BalanceHistoryService;

pub struct BalanceStatisticsQueryService {
    account_repo: Arc<dyn AccountRepository>,
    provider_repo: Arc<dyn ProviderRepository>,
    balance_history_service: Arc<BalanceHistoryService>,
}

impl BalanceStatisticsQueryService {
    pub fn new(
        account_repo: Arc<dyn AccountRepository>,
        provider_repo: Arc<dyn ProviderRepository>,
        balance_history_service: Arc<BalanceHistoryService>,
    ) -> Self {
        Self {
            account_repo,
            provider_repo,
            balance_history_service,
        }
    }

    pub async fn get_balance_statistics(&self) -> Result<BalanceStatisticsDto, DomainError> {
        let accounts = self.account_repo.find_enabled().await?;
        let providers = self.provider_repo.find_all().await?;
        let providers_by_id = providers
            .iter()
            .map(|provider| (provider.id().as_str().to_string(), provider.name().to_string()))
            .collect::<HashMap<_, _>>();

        let mut provider_stats: HashMap<String, ProviderBalanceDto> = HashMap::new();
        let mut total_current_balance = 0.0;
        let mut total_consumed = 0.0;
        let mut total_income = 0.0;

        for account in accounts {
            let cached = match (
                account.current_balance(),
                account.total_consumed(),
                account.total_income(),
            ) {
                (Some(cb), Some(tc), Some(ti)) => Some((cb, tc, ti)),
                _ => None,
            };

            let balance = if let Some(balance) = cached {
                Some(balance)
            } else {
                let account_id = account.id().as_str();
                self.balance_history_service
                    .get_latest_balance(account_id)
                    .await?
                    .map(|balance| {
                        (
                            balance.current_balance,
                            balance.total_consumed,
                            balance.total_income,
                        )
                    })
            };

            let Some((current_balance, consumed, income)) = balance else {
                continue;
            };

            let provider_id = account.provider_id().as_str();
            let provider_name = providers_by_id
                .get(provider_id)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());

            let stat = provider_stats
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

        Ok(BalanceStatisticsDto {
            providers: provider_stats.into_values().collect(),
            total_current_balance,
            total_consumed,
            total_income,
        })
    }
}

