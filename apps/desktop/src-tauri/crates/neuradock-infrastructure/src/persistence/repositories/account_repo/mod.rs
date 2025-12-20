mod encryption;
mod migration;
mod mutations;
mod queries;
mod types;

use async_trait::async_trait;
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::security::EncryptionService;
use neuradock_domain::account::{Account, AccountRepository};
use neuradock_domain::shared::{AccountId, DomainError};

pub struct SqliteAccountRepository {
    pool: Arc<SqlitePool>,
    encryption: Arc<EncryptionService>,
}

impl SqliteAccountRepository {
    const SELECT_QUERY: &'static str = r#"
            SELECT
                a.id, a.name, a.provider_id, a.cookies, a.api_user, a.enabled,
                bh.latest_recorded_at as last_check_in, a.created_at, a.auto_checkin_enabled,
                a.auto_checkin_hour, a.auto_checkin_minute,
                s.last_login_at, s.token as session_token, s.expires_at as session_expires_at,
                b.last_checked_at as last_balance_check_at,
                b.current as current_balance,
                b.total_consumed,
                b.total_income
            FROM accounts a
            LEFT JOIN sessions s ON a.id = s.account_id
            LEFT JOIN balances b ON a.id = b.account_id
            LEFT JOIN (
                SELECT account_id, MAX(recorded_at) as latest_recorded_at
                FROM balance_history
                GROUP BY account_id
            ) bh ON a.id = bh.account_id
        "#;

    pub fn new(pool: Arc<SqlitePool>, encryption: Arc<EncryptionService>) -> Self {
        Self { pool, encryption }
    }
}

#[async_trait]
impl AccountRepository for SqliteAccountRepository {
    async fn save(&self, account: &Account) -> Result<(), DomainError> {
        self.save_impl(account).await
    }

    async fn find_by_id(&self, id: &AccountId) -> Result<Option<Account>, DomainError> {
        self.find_by_id_impl(id).await
    }

    async fn find_by_ids(&self, ids: &[AccountId]) -> Result<Vec<Account>, DomainError> {
        self.find_by_ids_impl(ids).await
    }

    async fn find_all(&self) -> Result<Vec<Account>, DomainError> {
        self.find_all_impl().await
    }

    async fn find_enabled(&self) -> Result<Vec<Account>, DomainError> {
        self.find_enabled_impl().await
    }

    async fn delete(&self, id: &AccountId) -> Result<(), DomainError> {
        self.delete_impl(id).await
    }
}
