use std::time::Instant;
use tracing::info;

use super::types::AccountRow;
use crate::persistence::RepositoryErrorMapper;
use neuradock_domain::account::Account;
use neuradock_domain::shared::{AccountId, DomainError};

impl super::SqliteAccountRepository {
    pub(super) async fn find_by_id_impl(&self, id: &AccountId) -> Result<Option<Account>, DomainError> {
        let start = Instant::now();

        let query = format!(
            r#"
            {}
            WHERE a.id = ?1
        "#,
            Self::SELECT_QUERY
        );

        let row: Option<AccountRow> = sqlx::query_as(&query)
            .bind(id.as_str())
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find account by ID"))?;

        let elapsed = start.elapsed();
        let found = row.is_some();
        info!(
            "📊 find_by_id({}): {:.2}ms, found: {}",
            id.as_str(),
            elapsed.as_secs_f64() * 1000.0,
            found
        );

        match row {
            Some(row) => Ok(Some(row.to_account(&self.encryption)?)),
            None => Ok(None),
        }
    }

    pub(super) async fn find_by_ids_impl(&self, ids: &[AccountId]) -> Result<Vec<Account>, DomainError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let start = Instant::now();
        let id_strings: Vec<String> = ids.iter().map(|id| id.as_str().to_string()).collect();

        // Build parameterized query with placeholders
        let placeholders = (1..=id_strings.len())
            .map(|i| format!("?{}", i))
            .collect::<Vec<_>>()
            .join(",");

        let query = format!(
            r#"
            {}
            WHERE a.id IN ({})
        "#,
            Self::SELECT_QUERY,
            placeholders
        );

        // Build query dynamically with bindings
        let mut query_builder = sqlx::query_as::<_, AccountRow>(&query);
        for id_str in &id_strings {
            query_builder = query_builder.bind(id_str);
        }

        let rows: Vec<AccountRow> = query_builder
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find accounts by IDs"))?;

        let elapsed = start.elapsed();
        info!(
            "📊 find_by_ids({} ids): {:.2}ms, found: {} accounts",
            ids.len(),
            elapsed.as_secs_f64() * 1000.0,
            rows.len()
        );

        rows.into_iter()
            .map(|row| row.to_account(&self.encryption))
            .collect()
    }

    pub(super) async fn find_all_impl(&self) -> Result<Vec<Account>, DomainError> {
        let start = Instant::now();

        let query = format!(
            r#"
            {}
            ORDER BY a.created_at DESC
        "#,
            Self::SELECT_QUERY
        );

        let rows: Vec<AccountRow> = sqlx::query_as(&query)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find all accounts"))?;

        let elapsed = start.elapsed();
        let count = rows.len();

        if elapsed.as_millis() > 100 {
            tracing::warn!(
                "🐌 SLOW QUERY: find_all() took {:.2}ms for {} accounts",
                elapsed.as_secs_f64() * 1000.0,
                count
            );
        }

        // Graceful degradation: if one account fails to load (e.g. decryption error),
        // log it and continue with the others.
        let accounts = rows
            .into_iter()
            .filter_map(|row| match row.to_account(&self.encryption) {
                Ok(account) => Some(account),
                Err(e) => {
                    tracing::error!("Failed to load account: {}", e);
                    None
                }
            })
            .collect();

        info!(
            "📊 find_all(): {:.2}ms, {} accounts loaded",
            elapsed.as_secs_f64() * 1000.0,
            count
        );

        Ok(accounts)
    }

    pub(super) async fn find_enabled_impl(&self) -> Result<Vec<Account>, DomainError> {
        let start = Instant::now();

        let query = format!(
            r#"
            {}
            WHERE a.enabled = true
            ORDER BY a.created_at DESC
        "#,
            Self::SELECT_QUERY
        );

        let rows: Vec<AccountRow> = sqlx::query_as(&query)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| RepositoryErrorMapper::map_sqlx_error(e, "Find enabled accounts"))?;

        let elapsed = start.elapsed();
        let count = rows.len();

        let accounts = rows
            .into_iter()
            .filter_map(|row| match row.to_account(&self.encryption) {
                Ok(account) => Some(account),
                Err(e) => {
                    tracing::error!("Failed to load enabled account: {}", e);
                    None
                }
            })
            .collect();

        info!(
            "📊 find_enabled(): {:.2}ms, {} accounts loaded",
            elapsed.as_secs_f64() * 1000.0,
            count
        );

        Ok(accounts)
    }
}
