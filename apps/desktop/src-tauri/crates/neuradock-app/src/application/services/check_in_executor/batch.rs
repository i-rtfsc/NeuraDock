use anyhow::Result;
use log::{error, warn};
use std::collections::HashMap;
use tracing::instrument;

use neuradock_domain::{
    account::Account,
    check_in::Provider,
    shared::AccountId,
};

use super::types::{AccountCheckInResult, BatchCheckInResult};
use super::CheckInExecutor;

impl CheckInExecutor {
    /// Execute batch check-in for multiple accounts
    #[instrument(skip(self, providers), fields(batch_size = account_ids.len()))]
    pub async fn execute_batch_check_in(
        &self,
        account_ids: Vec<String>,
        providers: &HashMap<String, Provider>,
    ) -> Result<BatchCheckInResult> {
        let total = account_ids.len();
        let mut results = Vec::new();
        let mut success_count = 0;
        let mut failed_count = 0;

        // Batch load all accounts at once to avoid N+1 queries
        let account_id_objs: Vec<AccountId> = account_ids
            .iter()
            .map(|id| AccountId::from_string(id))
            .collect();

        let accounts = match self.account_repo.find_by_ids(&account_id_objs).await {
            Ok(accs) => accs,
            Err(e) => {
                error!("Failed to batch load accounts: {}", e);
                return Err(anyhow::anyhow!("Failed to load accounts: {}", e));
            }
        };

        // Create a map for quick lookup
        let account_map: HashMap<String, Account> = accounts
            .into_iter()
            .map(|acc| (acc.id().as_str().to_string(), acc))
            .collect();

        for account_id in account_ids {
            // Get account from pre-loaded map
            let account = match account_map.get(&account_id) {
                Some(acc) => acc,
                None => {
                    warn!("Account {} not found", account_id);
                    failed_count += 1;
                    continue;
                }
            };

            let provider_id = account.provider_id().as_str();
            let provider = match providers.get(provider_id) {
                Some(p) => p,
                None => {
                    warn!(
                        "Provider {} not found for account {}",
                        provider_id, account_id
                    );
                    failed_count += 1;
                    continue;
                }
            };

            // Execute check-in
            match self.execute_check_in(&account_id, provider).await {
                Ok(result) => {
                    if result.success {
                        success_count += 1;
                    } else {
                        failed_count += 1;
                    }
                    results.push(result);
                }
                Err(e) => {
                    error!("Check-in failed for account {}: {}", account_id, e);
                    failed_count += 1;
                    results.push(AccountCheckInResult {
                        account_id: account_id.clone(),
                        account_name: account.name().to_string(),
                        success: false,
                        message: format!("Error: {}", e),
                        user_info: None,
                    });
                }
            }
        }

        Ok(BatchCheckInResult {
            total,
            success_count,
            failed_count,
            results,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_check_in_result_calculation() {
        let results = vec![
            AccountCheckInResult {
                account_id: "1".to_string(),
                account_name: "Account 1".to_string(),
                success: true,
                message: "OK".to_string(),
                user_info: None,
            },
            AccountCheckInResult {
                account_id: "2".to_string(),
                account_name: "Account 2".to_string(),
                success: false,
                message: "Failed".to_string(),
                user_info: None,
            },
            AccountCheckInResult {
                account_id: "3".to_string(),
                account_name: "Account 3".to_string(),
                success: true,
                message: "OK".to_string(),
                user_info: None,
            },
        ];

        let success_count = results.iter().filter(|r| r.success).count();
        let failed_count = results.iter().filter(|r| !r.success).count();

        assert_eq!(success_count, 2);
        assert_eq!(failed_count, 1);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_batch_check_in_result_structure() {
        let batch_result = BatchCheckInResult {
            total: 5,
            success_count: 3,
            failed_count: 2,
            results: vec![],
        };

        assert_eq!(batch_result.total, 5);
        assert_eq!(batch_result.success_count, 3);
        assert_eq!(batch_result.failed_count, 2);
        assert_eq!(
            batch_result.total,
            batch_result.success_count + batch_result.failed_count
        );
    }

    #[test]
    fn test_account_check_in_result_structure() {
        let result = AccountCheckInResult {
            account_id: "test-account-123".to_string(),
            account_name: "Test Account".to_string(),
            success: true,
            message: "Check-in successful".to_string(),
            user_info: None,
        };

        assert_eq!(result.account_id, "test-account-123");
        assert_eq!(result.account_name, "Test Account");
        assert!(result.success);
        assert_eq!(result.message, "Check-in successful");
        assert!(result.user_info.is_none());
    }
}
