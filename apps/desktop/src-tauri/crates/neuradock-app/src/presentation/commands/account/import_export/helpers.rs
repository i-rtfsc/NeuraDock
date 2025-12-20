use chrono::{Duration, Utc};
use neuradock_domain::session::{Session, SessionRepository, SessionTokenExtractor};
use neuradock_domain::shared::AccountId;
use std::collections::HashMap;
use std::sync::Arc;

use crate::presentation::error::CommandError;

const DEFAULT_SESSION_EXPIRATION_DAYS: i64 = 30;

/// Helper function to create and save a default session for an account
pub(super) async fn create_and_save_default_session(
    account_id: AccountId,
    cookies: &HashMap<String, String>,
    session_repo: &Arc<dyn SessionRepository>,
) -> Result<(), CommandError> {
    let session_token = SessionTokenExtractor::extract(cookies);

    let expires_at = Utc::now() + Duration::days(DEFAULT_SESSION_EXPIRATION_DAYS);
    let session = Session::new(account_id, session_token, expires_at).map_err(CommandError::from)?;

    session_repo.save(&session).await.map_err(CommandError::from)?;
    Ok(())
}

/// Helper function to import a single account
pub(super) async fn import_single_account(
    input: crate::application::dtos::ImportAccountInput,
    account_repo: &Arc<dyn neuradock_domain::account::AccountRepository>,
    session_repo: &Arc<dyn SessionRepository>,
) -> Result<String, CommandError> {
    use neuradock_domain::account::{Account, Credentials};
    use neuradock_domain::shared::ProviderId;

    let cookies = input.cookies.clone();
    let credentials = Credentials::new(input.cookies, input.api_user);
    let account = Account::new(
        input.name,
        ProviderId::from_string(&input.provider),
        credentials,
    )
    .map_err(CommandError::from)?;

    let account_id = account.id().clone();
    account_repo.save(&account).await.map_err(CommandError::from)?;

    create_and_save_default_session(account_id.clone(), &cookies, session_repo).await?;

    Ok(account_id.as_str().to_string())
}

/// Helper function to update account cookies
pub(super) async fn update_account_cookies(
    account_id: &AccountId,
    cookies: HashMap<String, String>,
    api_user: String,
    account_repo: &Arc<dyn neuradock_domain::account::AccountRepository>,
    session_repo: &Arc<dyn SessionRepository>,
) -> Result<(), CommandError> {
    use neuradock_domain::account::Credentials;

    let mut account = account_repo
        .find_by_id(account_id)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found("Account not found"))?;

    let credentials = Credentials::new(cookies.clone(), api_user);
    account
        .update_credentials(credentials)
        .map_err(CommandError::from)?;
    account_repo.save(&account).await.map_err(CommandError::from)?;

    create_and_save_default_session(account_id.clone(), &cookies, session_repo).await?;

    Ok(())
}
