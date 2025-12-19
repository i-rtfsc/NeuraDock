use anyhow::Result;
use neuradock_domain::shared::AccountId;
use neuradock_domain::token::ApiToken;

impl super::TokenService {
    /// Get cached tokens only
    pub async fn get_cached_tokens(&self, account_id: &AccountId) -> Result<Vec<ApiToken>> {
        self.token_repo
            .find_by_account(account_id)
            .await
            .map_err(Into::into)
    }
}
