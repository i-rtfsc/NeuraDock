use super::*;
use neuradock_domain::account::{Account, Credentials};
use neuradock_domain::balance_history::BalanceHistoryRepository;
use neuradock_domain::shared::ProviderId;
use neuradock_infrastructure::persistence::repositories::{
    SqliteAccountRepository, SqliteBalanceHistoryRepository,
};
use neuradock_infrastructure::persistence::Database;
use neuradock_infrastructure::security::EncryptionService;
use std::collections::HashMap;

fn test_provider(id: &str, domain: &str) -> Provider {
    Provider::builtin(
        id,
        neuradock_domain::check_in::ProviderConfig {
            name: id.to_string(),
            domain: domain.to_string(),
            login_path: "/login".to_string(),
            sign_in_path: Some("/api/checkin".to_string()),
            user_info_path: "/self".to_string(),
            token_api_path: None,
            models_path: None,
            api_user_key: "new-api-user".to_string(),
            bypass_method: None,
            supports_check_in: true,
            check_in_bugged: false,
        },
    )
}

struct RewardFixture {
    _dir: tempfile::TempDir,
    pool: Arc<sqlx::SqlitePool>,
    account_repo: Arc<dyn AccountRepository>,
    history_repo: Arc<SqliteBalanceHistoryRepository>,
    account: Account,
}

impl RewardFixture {
    async fn new(provider_id: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let database = Database::new(dir.path().join("rewards.db").to_str().unwrap())
            .await
            .unwrap();
        database.run_migrations().await.unwrap();
        let pool = Arc::new(database.pool().clone());
        let account_repo: Arc<dyn AccountRepository> = Arc::new(SqliteAccountRepository::new(
            pool.clone(),
            Arc::new(EncryptionService::from_key(&[9; 32]).unwrap()),
        ));
        let account = Account::new(
            "Reward test".to_string(),
            ProviderId::from_string(provider_id),
            Credentials::new(
                HashMap::from([("session".to_string(), "test-session".to_string())]),
                "test-user".to_string(),
            ),
        )
        .unwrap();
        account_repo.save(&account).await.unwrap();
        Self {
            _dir: dir,
            pool: pool.clone(),
            account_repo,
            history_repo: Arc::new(SqliteBalanceHistoryRepository::new(pool)),
            account,
        }
    }

    fn history_service(&self) -> Arc<BalanceHistoryService> {
        Arc::new(BalanceHistoryService::new(self.history_repo.clone()))
    }
}

async fn mock_provider(fail_balance: bool) -> (String, tokio::task::JoinHandle<()>) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let posts = Arc::new(AtomicUsize::new(0));
    let task = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let posts = posts.clone();
            tokio::spawn(async move {
                let mut request = Vec::new();
                let mut buffer = [0u8; 2048];
                while !request.windows(4).any(|chunk| chunk == b"\r\n\r\n") {
                    let count = stream.read(&mut buffer).await.unwrap();
                    if count == 0 {
                        return;
                    }
                    request.extend_from_slice(&buffer[..count]);
                    if request.len() > 16384 {
                        return;
                    }
                }
                let body = if request.starts_with(b"POST ") {
                    posts.fetch_add(1, Ordering::SeqCst);
                    serde_json::json!({"success":true,"data":{"quota_awarded":12_500_000}})
                } else if posts.load(Ordering::SeqCst) == 0 {
                    serde_json::json!({"data":{"quota":50_000_000,"used_quota":100_000_000}})
                } else if fail_balance {
                    serde_json::json!({"data":{}})
                } else {
                    // A concurrent request has reserved 1.56, not yet reflected in used_quota.
                    serde_json::json!({"data":{"quota":61_720_000,"used_quota":100_000_000}})
                }
                .to_string();
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
                stream.write_all(response.as_bytes()).await.unwrap();
            });
        }
    });
    (address, task)
}

#[tokio::test]
async fn rewards_remain_25_through_concurrent_check_ins_refresh_calendar_and_notification() {
    use chrono::Datelike;
    use neuradock_domain::balance_history::BalanceHistoryRecord;
    use neuradock_infrastructure::persistence::repositories::SqliteProviderRepository;
    let f = RewardFixture::new("test-provider").await;
    let (address, server) = mock_provider(false).await;
    let provider = test_provider("test-provider", &address);
    let id = f.account.id().as_str();
    let now = chrono::Utc::now();
    f.history_repo
        .save(
            &BalanceHistoryRecord::new(
                "yesterday".to_string(),
                f.account.id().clone(),
                100.0,
                200.0,
                300.0,
                now - chrono::Duration::days(1),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let executor = CheckInExecutor::with_proxy(f.account_repo.clone(), true, None)
        .unwrap()
        .with_balance_history(f.history_service());
    let (first, second) = tokio::join!(
        executor.execute_check_in(id, &provider),
        executor.execute_check_in(id, &provider)
    );
    let first = first.unwrap();
    let second = second.unwrap();
    assert!(first.success, "{}", first.message);
    assert!(second.success, "{}", second.message);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM check_in_records WHERE account_id=?")
        .bind(id)
        .fetch_one(f.pool.as_ref())
        .await
        .unwrap();
    assert_eq!(count, 1);

    let queries = crate::application::queries::CheckInStreakQueries::new(
        f.account_repo.clone(),
        Arc::new(SqliteProviderRepository::new(f.pool.clone())),
        f.history_repo.clone(),
    );
    let day = queries
        .get_day_detail(id, &now.format("%Y-%m-%d").to_string())
        .await
        .unwrap();
    assert_eq!(day.income_increment, Some(25.0));
    assert!(!day.reward_is_estimate);
    assert_eq!(
        queries.get_trend(id, 1).await.unwrap().data_points[0].income_increment,
        Some(25.0)
    );
    assert_eq!(
        queries
            .get_calendar(id, now.year(), now.month())
            .await
            .unwrap()
            .month_stats
            .total_quota_increment,
        25.0
    );
    assert_eq!(
        queries
            .get_streak_stats(id)
            .await
            .unwrap()
            .total_check_in_days,
        1
    );

    let notifications = crate::application::services::NotificationService::new(
        Arc::new(
            neuradock_infrastructure::notification::SqliteNotificationChannelRepository::new(
                f.pool.clone(),
            ),
        ),
        f.history_repo.clone(),
    );
    let message = notifications
        .build_check_in_success_message(
            id,
            "Reward test",
            "Test provider",
            Some((123.44, 200.0, 323.44)),
        )
        .await
        .unwrap();
    assert!(message.content.starts_with(&format!(
        "{}: $25.00",
        crate::application::services::i18n::t("notification.label.checkInReward")
    )));

    // The first snapshot in a one-day chart is the reward, not lifetime quota.
    f.history_service()
        .save_balance_history(
            id,
            &crate::application::dtos::BalanceDto {
                current_balance: 900.0,
                total_consumed: 400.0,
                total_quota: 1300.0,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        queries
            .get_day_detail(id, &now.format("%Y-%m-%d").to_string())
            .await
            .unwrap()
            .income_increment,
        Some(25.0)
    );
    server.abort();
    f.pool.close().await;
}

#[tokio::test]
async fn a_failed_post_check_in_balance_query_does_not_lose_the_reward_or_write_old_data() {
    let f = RewardFixture::new("test-provider").await;
    let (address, server) = mock_provider(true).await;
    let provider = test_provider("test-provider", &address);
    let result = CheckInExecutor::with_proxy(f.account_repo.clone(), true, None)
        .unwrap()
        .with_balance_history(f.history_service())
        .execute_check_in(f.account.id().as_str(), &provider)
        .await
        .unwrap();
    assert!(result.success, "{}", result.message);
    assert!(result.user_info.is_none());
    let daily = f
        .history_repo
        .find_daily_summary(f.account.id(), chrono::Utc::now().date_naive())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(daily.check_in_reward(), Some(25.0));
    assert!(f
        .history_repo
        .find_latest_by_account_id(f.account.id())
        .await
        .unwrap()
        .is_none());
    server.abort();
    f.pool.close().await;
}

#[tokio::test]
async fn manual_confirmation_is_limited_to_known_rules_and_is_idempotent() {
    let f = RewardFixture::new("agentrouter").await;
    let service = f.history_service();
    let provider = test_provider("agentrouter", "https://agentrouter.org");
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let id = f.account.id().as_str();
    assert!(service
        .confirm_fixed_check_in(id, &today, &provider)
        .await
        .unwrap());
    assert!(!service
        .confirm_fixed_check_in(id, &today, &provider)
        .await
        .unwrap());
    let future = (chrono::Utc::now() + chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    assert!(service
        .confirm_fixed_check_in(id, &future, &provider)
        .await
        .is_err());
    assert!(service
        .confirm_fixed_check_in(id, &today, &test_provider("custom", "https://example.com"))
        .await
        .is_err());
    f.pool.close().await;
}

#[test]
fn fixed_rules_do_not_override_explicit_rewards_or_apply_to_unknown_providers() {
    let response = CheckInResult {
        success: true,
        message: "Confirmed".to_string(),
        reward: None,
    };
    assert_eq!(
        confirmed_reward(
            &test_provider("anyrouter", "https://anyrouter.top"),
            &response
        ),
        (Some(25.0), "provider_rule")
    );
    assert_eq!(
        confirmed_reward(
            &test_provider("agentrouter", "https://agentrouter.org"),
            &response
        ),
        (Some(25.0), "provider_rule")
    );
    assert_eq!(
        confirmed_reward(
            &test_provider("anyrouter", "https://example.com"),
            &response
        ),
        (None, "confirmed_unknown")
    );
    let response = CheckInResult {
        reward: Some(7.5),
        ..response
    };
    assert_eq!(
        confirmed_reward(
            &test_provider("anyrouter", "https://anyrouter.top"),
            &response
        ),
        (Some(7.5), "provider_response")
    );
}

#[tokio::test]
async fn scheduled_check_in_preserves_quota_corrections_in_history() {
    let dir = tempfile::tempdir().unwrap();
    let database = Database::new(dir.path().join("check-in.db").to_str().unwrap())
        .await
        .unwrap();
    database.run_migrations().await.unwrap();
    let pool = Arc::new(database.pool().clone());
    let encryption = Arc::new(EncryptionService::from_key(&[7; 32]).unwrap());
    let account_repo: Arc<dyn AccountRepository> =
        Arc::new(SqliteAccountRepository::new(pool.clone(), encryption));
    let history_repo = Arc::new(SqliteBalanceHistoryRepository::new(pool.clone()));

    let account = Account::new(
        "Corrected account".to_string(),
        ProviderId::from_string("test-provider"),
        Credentials::new(
            HashMap::from([("session".to_string(), "test-session".to_string())]),
            "test-user".to_string(),
        ),
    )
    .unwrap();
    account_repo.save(&account).await.unwrap();
    // Reproduce a correction already applied to the persisted account.
    sqlx::query("UPDATE accounts SET quota_adjustment = 0.02 WHERE id = ?")
        .bind(account.id().as_str())
        .execute(pool.as_ref())
        .await
        .unwrap();
    let mut account = account_repo
        .find_by_id(account.id())
        .await
        .unwrap()
        .unwrap();
    let executor = CheckInExecutor::with_proxy(account_repo.clone(), true, None)
        .unwrap()
        .with_balance_history(Arc::new(BalanceHistoryService::new(history_repo.clone())));
    let user_info = UserInfo {
        current_balance: 40.0,
        total_consumed: 59.98,
        total_quota: 99.98,
    };

    // Repeated saves on the same day must preserve, rather than remove or
    // double-apply, the adjustment. No provider request is made by this test.
    for _ in 0..2 {
        executor
            .record_check_in(&mut account, "Corrected account", Some(&user_info))
            .await;
        let history = history_repo
            .find_latest_by_account_id(account.id())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(history.total_quota(), 100.0);
        assert_eq!(history.current_balance(), 40.0);
        assert_eq!(history.total_consumed(), 59.98);
    }
    let saved = account_repo
        .find_by_id(account.id())
        .await
        .unwrap()
        .unwrap();
    assert!(saved.last_check_in().is_some());
    assert_eq!(saved.quota_adjustment(), 0.02);
    pool.close().await;
}
