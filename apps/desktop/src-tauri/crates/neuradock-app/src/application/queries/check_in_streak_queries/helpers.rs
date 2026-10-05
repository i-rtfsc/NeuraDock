use std::collections::HashMap;

use neuradock_domain::account::AccountRepository;
use neuradock_domain::balance_history::{BalanceHistoryDailySummary, BalanceHistoryRepository};
use neuradock_domain::check_in::{Provider, ProviderRepository, DEFAULT_DAY_OFFSET_MINUTES};
use neuradock_domain::shared::{AccountId, DomainError};

pub struct AccountInfo {
    pub account_id: String,
    pub name: String,
    pub provider_id: String,
    pub provider_name: String,
    /// Minutes east of UTC at which the provider resets its quota day. Needed
    /// wherever "today" is compared against an evidence day.
    pub provider_day_offset_minutes: i32,
}

/// Get account metadata (name + provider)
pub async fn get_account_info(
    account_repo: &dyn AccountRepository,
    provider_repo: &dyn ProviderRepository,
    account_id: &str,
) -> Result<AccountInfo, DomainError> {
    let account = account_repo
        .find_by_id(&AccountId::from_string(account_id))
        .await?
        .ok_or_else(|| DomainError::AccountNotFound(account_id.to_string()))?;

    let provider = provider_repo.find_by_id(account.provider_id()).await?;
    let provider_name = provider
        .as_ref()
        .map(|p| p.name().to_string())
        .unwrap_or_else(|| account.provider_id().as_str().to_string());

    Ok(AccountInfo {
        account_id: account.id().as_str().to_string(),
        name: account.name().to_string(),
        provider_id: account.provider_id().as_str().to_string(),
        provider_name,
        provider_day_offset_minutes: provider
            .map(|p| p.check_in_day_offset_minutes())
            .unwrap_or(DEFAULT_DAY_OFFSET_MINUTES),
    })
}

pub async fn get_all_account_infos(
    account_repo: &dyn AccountRepository,
    provider_repo: &dyn ProviderRepository,
) -> Result<Vec<AccountInfo>, DomainError> {
    let accounts = account_repo.find_all().await?;
    let providers = provider_repo.find_all().await?;

    let provider_map: HashMap<String, Provider> = providers
        .into_iter()
        .map(|p| (p.id().as_str().to_string(), p))
        .collect();

    let mut infos = accounts
        .into_iter()
        .map(|account| {
            let provider_id = account.provider_id().as_str().to_string();
            let provider = provider_map.get(&provider_id);
            let provider_name = provider
                .map(|p| p.name().to_string())
                .unwrap_or_else(|| provider_id.clone());

            AccountInfo {
                account_id: account.id().as_str().to_string(),
                name: account.name().to_string(),
                provider_id,
                provider_name,
                provider_day_offset_minutes: provider
                    .map(|p| p.check_in_day_offset_minutes())
                    .unwrap_or(DEFAULT_DAY_OFFSET_MINUTES),
            }
        })
        .collect::<Vec<_>>();

    infos.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(infos)
}

pub async fn fetch_all_daily_summaries(
    balance_history_repo: &dyn BalanceHistoryRepository,
    account_id: &str,
) -> Result<Vec<BalanceHistoryDailySummary>, DomainError> {
    balance_history_repo
        .list_all_daily_summaries(&AccountId::from_string(account_id))
        .await
}
