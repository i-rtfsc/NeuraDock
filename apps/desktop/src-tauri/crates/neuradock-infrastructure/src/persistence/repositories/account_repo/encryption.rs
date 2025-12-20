use serde_json;

use crate::persistence::RepositoryErrorMapper;
use neuradock_domain::shared::DomainError;

impl super::SqliteAccountRepository {
    /// Encrypt plaintext account data
    pub(super) async fn encrypt_account_data(
        &self,
        account_id: &str,
        plaintext_cookies: &str,
        plaintext_api_user: &str,
    ) -> Result<(), DomainError> {
        // Validate that cookies is valid JSON
        let _: serde_json::Value = serde_json::from_str(plaintext_cookies)
            .map_err(|e| DomainError::Validation(format!("Invalid cookies JSON: {}", e)))?;

        // Encrypt the data
        let encrypted_cookies = self
            .encryption
            .encrypt(plaintext_cookies)
            .map_err(|e| DomainError::DataIntegrity(format!("Failed to encrypt cookies: {}", e)))?;

        let encrypted_api_user = self.encryption.encrypt(plaintext_api_user).map_err(|e| {
            DomainError::DataIntegrity(format!("Failed to encrypt api_user: {}", e))
        })?;

        // Update the database
        let update_query = "UPDATE accounts SET cookies = ?1, api_user = ?2 WHERE id = ?3";
        sqlx::query(update_query)
            .bind(encrypted_cookies)
            .bind(encrypted_api_user)
            .bind(account_id)
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                RepositoryErrorMapper::map_sqlx_error(e, "Update encrypted account data")
            })?;

        Ok(())
    }
}
