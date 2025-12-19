use chrono::{Duration, Utc};
use neuradock_domain::session::{Session, SessionRepository};
use neuradock_domain::shared::AccountId;
use std::collections::HashMap;
use std::sync::Arc;

use crate::application::ResultExt;

const DEFAULT_SESSION_EXPIRATION_DAYS: i64 = 30;

/// Helper function to create and save a default session for an account
pub(super) async fn create_and_save_default_session(
    account_id: AccountId,
    cookies: &HashMap<String, String>,
    session_repo: &Arc<dyn SessionRepository>,
) -> Result<(), String> {
    let session_token = cookies
        .values()
        .next()
        .cloned()
        .unwrap_or_else(|| "session".to_string());

    let expires_at = Utc::now() + Duration::days(DEFAULT_SESSION_EXPIRATION_DAYS);
    let session = Session::new(account_id, session_token, expires_at).to_string_err()?;

    session_repo.save(&session).await.to_string_err()?;
    Ok(())
}

/// Helper function to import a single account
pub(super) async fn import_single_account(
    input: crate::application::dtos::ImportAccountInput,
    account_repo: &Arc<dyn neuradock_domain::account::AccountRepository>,
    session_repo: &Arc<dyn SessionRepository>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    use neuradock_domain::account::{Account, Credentials};
    use neuradock_domain::shared::ProviderId;

    let cookies = input.cookies.clone();
    let credentials = Credentials::new(input.cookies, input.api_user);
    let account = Account::new(
        input.name,
        ProviderId::from_string(&input.provider),
        credentials,
    )?;

    let account_id = account.id().clone();
    account_repo.save(&account).await?;

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
) -> Result<(), Box<dyn std::error::Error>> {
    use neuradock_domain::account::Credentials;

    let mut account = account_repo
        .find_by_id(account_id)
        .await?
        .ok_or("Account not found")?;

    let credentials = Credentials::new(cookies.clone(), api_user);
    account.update_credentials(credentials)?;
    account_repo.save(&account).await?;

    create_and_save_default_session(account_id.clone(), &cookies, session_repo).await?;

    Ok(())
}
