use std::collections::HashMap;

use neuradock_domain::{account::Account, check_in::Provider};
use neuradock_infrastructure::http::{CheckInResult, SetCookieResult, UserInfo};

use crate::application::services::user_info_service::UserInfoService;

/// Fetch updated balance after successful check-in
pub async fn fetch_updated_balance_after_check_in(
    user_info_service: &UserInfoService<'_>,
    account: &Account,
    provider: &Provider,
    account_name: &str,
    cookies: &HashMap<String, String>,
    check_in_result: &CheckInResult,
    initial_user_info: Option<UserInfo>,
) -> (Option<UserInfo>, SetCookieResult) {
    // Only fetch updated balance if check-in was successful
    if !check_in_result.success {
        return (initial_user_info, SetCookieResult::default());
    }

    let api_user = account.credentials().api_user();

    user_info_service
        .fetch_updated_balance(account_name, provider, cookies, api_user, initial_user_info)
        .await
}
