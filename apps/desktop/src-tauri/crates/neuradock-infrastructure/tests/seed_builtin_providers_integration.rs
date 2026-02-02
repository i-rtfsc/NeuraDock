use std::collections::HashMap;
use std::sync::Arc;

use neuradock_domain::account::{Account, AccountRepository, Credentials};
use neuradock_domain::check_in::{Provider, ProviderConfig, ProviderRepository};
use neuradock_domain::custom_node::{CustomProviderNode, CustomProviderNodeRepository};
use neuradock_domain::provider_models::ProviderModelsRepository;
use neuradock_domain::shared::ProviderId;
use neuradock_domain::waf_cookies::WafCookiesRepository;
use neuradock_infrastructure::bootstrap::seed_builtin_providers;
use neuradock_infrastructure::persistence::repositories::{
    SqliteAccountRepository, SqliteCustomProviderNodeRepository, SqliteProviderModelsRepository,
    SqliteProviderRepository, SqliteWafCookiesRepository,
};

mod test_helpers;

#[tokio::test]
async fn seed_builtin_providers_migrates_stale_provider_references() {
    let (pool, encryption) = test_helpers::setup_in_memory_db().await;
    let pool = Arc::new(pool);

    let account_repo: Arc<dyn AccountRepository> =
        Arc::new(SqliteAccountRepository::new(pool.clone(), encryption.clone()));
    let provider_repo: Arc<dyn ProviderRepository> =
        Arc::new(SqliteProviderRepository::new(pool.clone()));
    let custom_node_repo: Arc<dyn CustomProviderNodeRepository> =
        Arc::new(SqliteCustomProviderNodeRepository::new(pool.clone()));
    let provider_models_repo: Arc<dyn ProviderModelsRepository> =
        Arc::new(SqliteProviderModelsRepository::new(pool.clone()));
    let waf_cookies_repo: Arc<dyn WafCookiesRepository> =
        Arc::new(SqliteWafCookiesRepository::new(pool.clone()));

    let stale_provider = Provider::new(ProviderConfig {
        name: "Wong".to_string(),
        domain: "https://wzw.pp.ua".to_string(),
        login_path: "/login".to_string(),
        sign_in_path: Some("/api/user/checkin".to_string()),
        user_info_path: "/api/user/self".to_string(),
        token_api_path: Some("/api/token/".to_string()),
        models_path: Some("/api/user/models".to_string()),
        api_user_key: "new-api-user".to_string(),
        bypass_method: None,
        supports_check_in: true,
        check_in_bugged: false,
    });
    let stale_provider_id = stale_provider.id().clone();
    provider_repo
        .save(&stale_provider)
        .await
        .expect("save stale provider");

    let mut cookies = HashMap::new();
    cookies.insert("session".to_string(), "abc123".to_string());
    let credentials = Credentials::new(cookies, "api_user_1".to_string());
    let account = Account::new(
        "Test Account".to_string(),
        stale_provider_id.clone(),
        credentials,
    )
    .expect("create account");
    account_repo.save(&account).await.expect("save account");

    let custom_node = CustomProviderNode::create(
        stale_provider_id.clone(),
        "Legacy Node".to_string(),
        "https://node.example".to_string(),
    );
    let _ = custom_node_repo
        .create(&custom_node)
        .await
        .expect("save custom node");

    provider_models_repo
        .save(stale_provider_id.as_str(), &vec!["model-a".to_string()])
        .await
        .expect("save provider models");

    let mut waf_cookies = HashMap::new();
    waf_cookies.insert("acw_sc__v2".to_string(), "cookie".to_string());
    waf_cookies_repo
        .save(stale_provider_id.as_str(), &waf_cookies)
        .await
        .expect("save waf cookies");

    seed_builtin_providers(
        provider_repo.clone(),
        custom_node_repo.clone(),
        account_repo.clone(),
        provider_models_repo.clone(),
        waf_cookies_repo.clone(),
    )
    .await
    .expect("seed builtin providers");

    let updated_account = account_repo
        .find_by_id(account.id())
        .await
        .expect("find account")
        .expect("account should exist");
    assert_eq!(updated_account.provider_id().as_str(), "wong");

    let stale_provider = provider_repo
        .find_by_id(&stale_provider_id)
        .await
        .expect("find stale provider");
    assert!(stale_provider.is_none());

    let builtin_provider = provider_repo
        .find_by_id(&ProviderId::from_string("wong"))
        .await
        .expect("find builtin provider");
    assert!(builtin_provider.is_some());

    let migrated_nodes = custom_node_repo
        .find_by_provider(&ProviderId::from_string("wong"))
        .await
        .expect("find custom nodes");
    assert!(migrated_nodes
        .iter()
        .any(|node| node.base_url() == "https://node.example"));

    let migrated_models = provider_models_repo
        .find_by_provider("wong")
        .await
        .expect("find provider models");
    assert!(migrated_models.is_some());

    let old_models = provider_models_repo
        .find_by_provider(stale_provider_id.as_str())
        .await
        .expect("find old provider models");
    assert!(old_models.is_none());

    let new_waf = waf_cookies_repo
        .get_valid("wong")
        .await
        .expect("get new waf cookies");
    assert!(new_waf.is_some());

    let old_waf = waf_cookies_repo
        .get_valid(stale_provider_id.as_str())
        .await
        .expect("get old waf cookies");
    assert!(old_waf.is_none());
}
