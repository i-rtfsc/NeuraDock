use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;

use crate::application::dtos::AccountDto;
use neuradock_domain::account::AccountRepository;
use neuradock_domain::check_in::Provider;
use neuradock_domain::shared::DomainError;

/// Account query service
/// Handles all read operations for accounts with optimized projections
pub struct AccountQueryService {
    account_repo: Arc<dyn AccountRepository>,
}

impl AccountQueryService {
    pub fn new(account_repo: Arc<dyn AccountRepository>) -> Self {
        Self { account_repo }
    }

    /// Get all accounts with optional filtering
    pub async fn get_all_accounts(
        &self,
        enabled_only: bool,
        providers: &HashMap<String, Provider>,
    ) -> Result<Vec<AccountDto>, DomainError> {
        let accounts = if enabled_only {
            self.account_repo.find_enabled().await?
        } else {
            self.account_repo.find_all().await?
        };

        use crate::application::dtos::AccountDtoMapper;

        let now = Utc::now();
        let dtos = accounts
            .iter()
            .map(|acc| {
                let provider_name = providers
                    .get(acc.provider_id().as_str())
                    .map(|p| p.name().to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                AccountDtoMapper::new(acc, provider_name)
                    .with_time(now)
                    .to_dto()
            })
            .collect();

        Ok(dtos)
    }

    /// Get a single account by ID
    pub async fn get_account_by_id(
        &self,
        account_id: &str,
        providers: &HashMap<String, Provider>,
    ) -> Result<Option<AccountDto>, DomainError> {
        use neuradock_domain::shared::AccountId;

        let account_id = AccountId::from_string(account_id);
        let account = self.account_repo.find_by_id(&account_id).await?;

        use crate::application::dtos::AccountDtoMapper;

        let now = Utc::now();
        Ok(account.map(|acc| {
            let provider_name = providers
                .get(acc.provider_id().as_str())
                .map(|p| p.name().to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            AccountDtoMapper::new(&acc, provider_name)
                .with_time(now)
                .to_dto()
        }))
    }

    /// Get enabled accounts only
    pub async fn get_enabled_accounts(
        &self,
        providers: &HashMap<String, Provider>,
    ) -> Result<Vec<AccountDto>, DomainError> {
        self.get_all_accounts(true, providers).await
    }

    /// Get account summary statistics
    pub async fn get_account_statistics(&self) -> Result<AccountStatistics, DomainError> {
        let all_accounts = self.account_repo.find_all().await?;
        let enabled_accounts = all_accounts.iter().filter(|a| a.is_enabled()).count();

        let total_balance: f64 = all_accounts
            .iter()
            .filter_map(|a| a.current_balance())
            .sum();

        let online_accounts = all_accounts
            .iter()
            .filter(|a| a.is_session_valid() || !a.is_balance_stale(24))
            .count();

        Ok(AccountStatistics {
            total_accounts: all_accounts.len(),
            enabled_accounts,
            disabled_accounts: all_accounts.len() - enabled_accounts,
            online_accounts,
            offline_accounts: all_accounts.len() - online_accounts,
            total_balance,
        })
    }
}

/// Account statistics view model
#[derive(Debug, Clone)]
pub struct AccountStatistics {
    pub total_accounts: usize,
    pub enabled_accounts: usize,
    pub disabled_accounts: usize,
    pub online_accounts: usize,
    pub offline_accounts: usize,
    pub total_balance: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use neuradock_domain::account::{Account, Credentials};
    use neuradock_domain::shared::{AccountId, ProviderId};
    use std::collections::HashMap;

    struct MockAccountRepository {
        accounts: Vec<Account>,
    }

    #[async_trait::async_trait]
    impl AccountRepository for MockAccountRepository {
        async fn find_all(&self) -> Result<Vec<Account>, DomainError> {
            Ok(self.accounts.clone())
        }

        async fn find_enabled(&self) -> Result<Vec<Account>, DomainError> {
            Ok(self
                .accounts
                .iter()
                .filter(|a| a.is_enabled())
                .cloned()
                .collect())
        }

        async fn find_by_id(
            &self,
            id: &AccountId,
        ) -> Result<Option<Account>, DomainError> {
            Ok(self.accounts.iter().find(|a| a.id() == id).cloned())
        }

        async fn find_by_ids(
            &self,
            ids: &[AccountId],
        ) -> Result<Vec<Account>, DomainError> {
            Ok(self
                .accounts
                .iter()
                .filter(|a| ids.iter().any(|id| a.id() == id))
                .cloned()
                .collect())
        }

        async fn save(&self, _account: &Account) -> Result<(), DomainError> {
            Ok(())
        }

        async fn delete(&self, _id: &AccountId) -> Result<(), DomainError> {
            Ok(())
        }
    }

    fn create_test_account(name: &str, enabled: bool) -> Account {
        let mut cookies = HashMap::new();
        cookies.insert("session".to_string(), "test_session".to_string());

        let mut account = Account::new(
            name.to_string(),
            ProviderId::new(),
            Credentials::new(cookies, "test@user".to_string()),
        )
        .unwrap();

        if !enabled {
            account.toggle(false);
        }

        account
    }

    #[tokio::test]
    async fn test_get_all_accounts() {
        let accounts = vec![
            create_test_account("Account 1", true),
            create_test_account("Account 2", false),
        ];

        let repo = Arc::new(MockAccountRepository {
            accounts: accounts.clone(),
        });

        let service = AccountQueryService::new(repo);
        let providers = HashMap::new();

        let result = service.get_all_accounts(false, &providers).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_get_enabled_accounts_only() {
        let accounts = vec![
            create_test_account("Account 1", true),
            create_test_account("Account 2", false),
        ];

        let repo = Arc::new(MockAccountRepository {
            accounts: accounts.clone(),
        });

        let service = AccountQueryService::new(repo);
        let providers = HashMap::new();

        let result = service.get_all_accounts(true, &providers).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Account 1");
    }

    #[tokio::test]
    async fn test_get_account_statistics() {
        let accounts = vec![
            create_test_account("Account 1", true),
            create_test_account("Account 2", false),
            create_test_account("Account 3", true),
        ];

        let repo = Arc::new(MockAccountRepository {
            accounts: accounts.clone(),
        });

        let service = AccountQueryService::new(repo);

        let stats = service.get_account_statistics().await.unwrap();
        assert_eq!(stats.total_accounts, 3);
        assert_eq!(stats.enabled_accounts, 2);
        assert_eq!(stats.disabled_accounts, 1);
    }
}
