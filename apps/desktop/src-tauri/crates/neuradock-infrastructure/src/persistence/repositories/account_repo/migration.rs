use tracing::{info, warn};

use crate::persistence::RepositoryErrorMapper;
use neuradock_domain::shared::{AccountId, DomainError};

impl super::SqliteAccountRepository {
    /// Migrate unencrypted accounts by attempting to detect and re-encrypt plaintext data
    ///
    /// ⚠️ WARNING: This method is for migration purposes only and should be called once
    /// during deployment. After migration, all accounts must be properly encrypted.
    ///
    /// Returns the IDs of accounts that were successfully migrated.
    pub async fn migrate_unencrypted_accounts(&self) -> Result<Vec<AccountId>, DomainError> {
        info!("🔄 Starting migration of unencrypted accounts...");

        // Query all raw account rows directly
        let query = "SELECT id, cookies, api_user FROM accounts";
        let rows: Vec<(String, String, String)> = sqlx::query_as(query)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| {
                RepositoryErrorMapper::map_sqlx_error(e, "Fetch accounts for migration")
            })?;

        let mut migrated_ids = Vec::new();
        let mut failed_accounts = Vec::new();

        for (id, cookies, api_user) in rows {
            // Try to decrypt - if it fails, assume it's plaintext and needs migration
            let needs_migration = self.encryption.decrypt(&cookies).is_err()
                || self.encryption.decrypt(&api_user).is_err();

            if needs_migration {
                info!("🔐 Migrating account: {}", id);

                // Assume the data is plaintext and try to encrypt it
                match self.encrypt_account_data(&id, &cookies, &api_user).await {
                    Ok(_) => {
                        migrated_ids.push(AccountId::from_string(&id));
                        info!("✅ Successfully migrated account: {}", id);
                    }
                    Err(e) => {
                        warn!("❌ Failed to migrate account {}: {}", id, e);
                        failed_accounts.push((id.clone(), e.to_string()));
                    }
                }
            }
        }

        if !failed_accounts.is_empty() {
            warn!(
                "⚠️  Migration completed with {} failures:",
                failed_accounts.len()
            );
            for (id, error) in &failed_accounts {
                warn!("  - Account {}: {}", id, error);
            }
        }

        info!(
            "✅ Migration completed. Migrated {} accounts",
            migrated_ids.len()
        );
        Ok(migrated_ids)
    }
}
