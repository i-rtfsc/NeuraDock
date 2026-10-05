//! Rehearses pending migrations against a copy of a real database.
//!
//! Migrations are the one thing tests cannot fully fake: the in-memory
//! fixtures start from an empty schema and build it forward, so a data
//! migration that re-buckets or de-duplicates existing rows is only exercised
//! by real rows. This test points `sqlx::migrate!` -- the same runner the app
//! uses at startup -- at a copy and then checks the invariants the ledger
//! depends on.
//!
//! Always run it against a copy, never the live file:
//!
//! ```text
//! sqlite3 "$LIVE_DB" ".backup /tmp/rehearsal.db"
//! NEURADOCK_REHEARSAL_DB=/tmp/rehearsal.db \
//!   cargo test -p neuradock-infrastructure --test migrate_real_db -- --ignored --nocapture
//! ```

use neuradock_infrastructure::persistence::Database;
use sqlx::{Row, SqlitePool};
use std::path::Path;

async fn count(pool: &SqlitePool, sql: &str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
}

#[tokio::test]
#[ignore = "needs NEURADOCK_REHEARSAL_DB pointing at a copy of a real database"]
async fn pending_migrations_apply_cleanly_to_a_real_database() {
    let path = std::env::var("NEURADOCK_REHEARSAL_DB")
        .expect("set NEURADOCK_REHEARSAL_DB to a *copy* of a real database");
    assert!(Path::new(&path).exists(), "{path} does not exist");

    let database = Database::new(&path).await.unwrap();
    database.run_migrations().await.unwrap();
    let pool = database.pool();

    // 1. The rebuild keeps at most one evidence row per account and quota day.
    let duplicates = count(
        pool,
        "SELECT COUNT(*) FROM (
             SELECT account_id, quota_day FROM check_in_records
             GROUP BY account_id, quota_day HAVING COUNT(*) > 1)",
    )
    .await;
    assert_eq!(duplicates, 0, "a day must hold at most one evidence row");

    // 2. Confidence is derivable from source and never disagrees.
    let mismatched = count(
        pool,
        "SELECT COUNT(*) FROM check_in_records WHERE confidence IS NOT
             CASE source
                 WHEN 'manual' THEN 'manual'
                 WHEN 'legacy_estimate' THEN 'inferred'
                 WHEN 'balance_delta' THEN 'inferred'
                 ELSE 'confirmed'
             END",
    )
    .await;
    assert_eq!(mismatched, 0, "confidence must match its source");

    // 3. Every quota day is a usable date (an unparseable one would break the
    //    calendar and the trend for the whole account).
    let unparseable = count(
        pool,
        "SELECT COUNT(*) FROM check_in_records
         WHERE quota_day IS NULL OR date(quota_day) IS NULL",
    )
    .await;
    assert_eq!(unparseable, 0, "quota_day must parse as a date");

    // 4. The account cache is derivable from the ledger it caches.
    let cache_drift = count(
        pool,
        "SELECT COUNT(*) FROM accounts a
         WHERE COALESCE(a.last_evidence_at, '') != COALESCE(
             (SELECT MAX(recorded_at) FROM check_in_records c WHERE c.account_id = a.id), '')
            OR COALESCE(a.last_confirmed_at, '') != COALESCE(
             (SELECT MAX(recorded_at) FROM check_in_records c
               WHERE c.account_id = a.id AND c.confidence IN ('confirmed', 'manual')), '')",
    )
    .await;
    assert_eq!(cache_drift, 0, "the cache must agree with the ledger");

    // 5. Providers are classified, so nothing is left guessing its mode.
    let unclassified = count(
        pool,
        "SELECT COUNT(*) FROM providers
         WHERE check_in_mode IS NULL OR check_in_day_offset_minutes IS NULL",
    )
    .await;
    assert_eq!(
        unclassified, 0,
        "every provider needs a mode and a date offset"
    );

    // 5b. No phantom legacy days: a legacy row with no amount had no quota
    //     increase to infer a check-in from.
    let phantoms = count(
        pool,
        "SELECT COUNT(*) FROM check_in_records
         WHERE source = 'legacy_estimate' AND reward_cents IS NULL",
    )
    .await;
    assert_eq!(phantoms, 0, "an inference needs an increase to infer from");

    // 6. Attempts are append-only telemetry and must not have been touched.
    let attempts = count(pool, "SELECT COUNT(*) FROM check_in_attempts").await;

    println!("\n--- rehearsal summary ({path}) ---");
    for row in sqlx::query(
        "SELECT confidence, COUNT(*) AS rows FROM check_in_records
         GROUP BY confidence ORDER BY confidence",
    )
    .fetch_all(pool)
    .await
    .unwrap()
    {
        println!(
            "evidence {:>10}: {}",
            row.get::<String, _>("confidence"),
            row.get::<i64, _>("rows")
        );
    }
    for row in sqlx::query(
        "SELECT check_in_mode, COUNT(*) AS rows FROM providers
         GROUP BY check_in_mode ORDER BY check_in_mode",
    )
    .fetch_all(pool)
    .await
    .unwrap()
    {
        println!(
            "provider mode {:>13}: {}",
            row.get::<String, _>("check_in_mode"),
            row.get::<i64, _>("rows")
        );
    }
    let with_evidence = count(
        pool,
        "SELECT COUNT(DISTINCT account_id) FROM check_in_records",
    )
    .await;
    let with_cache = count(
        pool,
        "SELECT COUNT(*) FROM accounts WHERE last_evidence_at IS NOT NULL",
    )
    .await;
    println!("accounts with evidence: {with_evidence}, with a cached date: {with_cache}");
    println!("attempts: {attempts}");
    println!("--- done ---\n");
}
