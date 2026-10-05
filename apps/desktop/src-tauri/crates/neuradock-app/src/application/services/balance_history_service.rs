use crate::application::dtos::BalanceDto;
use chrono::{DateTime, NaiveDate, Utc};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tracing::{debug, info, warn};

use neuradock_domain::balance_history::{
    AttemptCompletion, BalanceHistoryRecord, BalanceHistoryRepository, CheckInAttempt,
    CheckInConfidence, CheckInOutcome, CheckInRecord, CheckInSource, CheckInTrigger,
};
use neuradock_domain::check_in::{
    evaluate_check_in_state, next_quota_day_start, quota_day_of, today_quota_day, CheckInPolicy,
    CheckInState, Provider,
};
use neuradock_domain::shared::{AccountId, DomainError};

/// Below this, a quota increase is treated as rounding drift rather than a
/// check-in. `total_quota` is derived from two rounded values, so an untouched
/// day can still read a cent or two higher; real rewards are whole dollars.
const MIN_INFERRED_DELTA: f64 = 0.05;

/// The ledger's application boundary.
///
/// Every balance observation and every check-in goes through here so the three
/// facts the app needs stay consistent: what the provider reported over time
/// (snapshots), what the app knows happened (evidence), and what it tried
/// (attempts).
pub struct BalanceHistoryService {
    repository: Arc<dyn BalanceHistoryRepository>,
    policy: CheckInPolicy,
}

impl BalanceHistoryService {
    pub fn new(repository: Arc<dyn BalanceHistoryRepository>) -> Self {
        Self {
            repository,
            policy: CheckInPolicy::default(),
        }
    }

    fn account(account_id: &str) -> AccountId {
        AccountId::from_string(account_id)
    }

    fn day_key(day: NaiveDate) -> String {
        day.format("%Y-%m-%d").to_string()
    }

    /// One snapshot per account per quota day, so a later refresh replaces the
    /// earlier reading instead of piling up rows.
    fn snapshot_id(account_id: &str, quota_day: NaiveDate) -> String {
        let mut hasher = Sha256::new();
        hasher.update(account_id.as_bytes());
        hasher.update(Self::day_key(quota_day).as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Persist one balance observation: the snapshot always, plus inferred
    /// evidence when the observation proves a check-in happened.
    ///
    /// This is the runtime half of "a web check-in is detected by looking at the
    /// balance": a provider the app cannot post to still gets its day recorded
    /// the next time the balance is read.
    pub async fn save_balance_history(
        &self,
        account_id: &str,
        provider: &Provider,
        balance: &BalanceDto,
    ) -> Result<(), DomainError> {
        let now = Utc::now();
        let quota_day = quota_day_of(now, provider.check_in_day_offset_minutes());

        let snapshot = BalanceHistoryRecord::new(
            Self::snapshot_id(account_id, quota_day),
            Self::account(account_id),
            balance.current_balance,
            balance.total_consumed,
            balance.total_quota,
            now,
        )?;

        let inferred = self
            .infer_evidence(account_id, quota_day, balance.total_quota, now)
            .await?;

        if let Some(record) = &inferred {
            info!(
                account_id,
                day = %quota_day,
                delta = ?record.reward(),
                "Inferred a check-in from a quota increase"
            );
        }

        self.repository
            .save_balance_observation(&snapshot, inferred.as_ref())
            .await?;

        debug!(
            account_id,
            current_balance = balance.current_balance,
            total_consumed = balance.total_consumed,
            total_quota = balance.total_quota,
            inferred = inferred.is_some(),
            "Balance observation stored"
        );

        Ok(())
    }

    /// Evidence implied by an observation, or `None` when it proves nothing.
    ///
    /// No baseline means no evidence: the first snapshot of an account has
    /// nothing to compare against, and inventing a check-in there is how the
    /// legacy backfill produced phantom days.
    async fn infer_evidence(
        &self,
        account_id: &str,
        quota_day: NaiveDate,
        total_quota: f64,
        now: DateTime<Utc>,
    ) -> Result<Option<CheckInRecord>, DomainError> {
        let Some(previous_day) = quota_day.pred_opt() else {
            return Ok(None);
        };

        let Some(previous) = self
            .repository
            .find_daily_summary(&Self::account(account_id), previous_day)
            .await?
        else {
            return Ok(None);
        };

        let delta = total_quota - previous.daily_total_quota();
        if delta < MIN_INFERRED_DELTA {
            return Ok(None);
        }

        Ok(Some(CheckInRecord::new(
            Self::account(account_id),
            quota_day,
            now,
            Some(delta),
            CheckInSource::BalanceDelta,
        )?))
    }

    /// Record a reward the provider confirmed, for the quota day that contains
    /// `recorded_at`.
    pub async fn save_check_in_reward(
        &self,
        account_id: &str,
        provider: &Provider,
        recorded_at: DateTime<Utc>,
        reward: Option<f64>,
        source: CheckInSource,
        attempt_id: Option<i64>,
    ) -> Result<bool, DomainError> {
        let quota_day = quota_day_of(recorded_at, provider.check_in_day_offset_minutes());
        let record = CheckInRecord::new(
            Self::account(account_id),
            quota_day,
            recorded_at,
            reward,
            source,
        )?
        .with_attempt_id(attempt_id);

        self.repository.save_check_in(&record).await
    }

    /// Record a check-in the user completed outside the app.
    ///
    /// Allowed for providers whose reward the app knows (`fixed_reward`), which
    /// is exactly the set it cannot post to. A settled amount is left alone, and
    /// replacing an estimate is audited through an attempt row.
    pub async fn confirm_fixed_check_in(
        &self,
        account_id: &str,
        date: &str,
        provider: &Provider,
    ) -> Result<bool, DomainError> {
        let quota_day = NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| DomainError::Validation("Invalid check-in date".to_string()))?;
        let now = Utc::now();
        if quota_day > today_quota_day(provider.check_in_day_offset_minutes(), now) {
            return Err(DomainError::Validation(
                "Cannot confirm a future check-in".to_string(),
            ));
        }

        let reward = super::check_in_executor::fixed_reward(provider).ok_or_else(|| {
            DomainError::Validation("This provider has no confirmed fixed reward".to_string())
        })?;

        let id = Self::account(account_id);
        if let Some(existing) = self.repository.find_evidence(&id, quota_day).await? {
            if existing.confidence() != CheckInConfidence::Inferred
                && existing.reward_cents().is_some()
            {
                return Ok(false);
            }
            info!(
                account_id,
                day = %quota_day,
                before = existing.source().as_str(),
                after = CheckInSource::Manual.as_str(),
                "Replacing estimated check-in evidence with a manual confirmation"
            );
        }

        let attempt = CheckInAttempt::start(
            id.clone(),
            provider.id().clone(),
            quota_day,
            CheckInTrigger::Manual,
            now,
        );
        let attempt_id = self.repository.start_attempt(&attempt).await?;

        let record = CheckInRecord::new(id, quota_day, now, Some(reward), CheckInSource::Manual)?
            .with_attempt_id(Some(attempt_id));
        let changed = self.repository.save_check_in(&record).await?;

        self.repository
            .finish_attempt(
                attempt_id,
                &AttemptCompletion {
                    outcome: CheckInOutcome::AlreadyConfirmed,
                    finished_at: Utc::now(),
                    error_code: None,
                    http_status: None,
                    reward: Some(reward),
                    source: Some(CheckInSource::Manual),
                },
            )
            .await?;

        Ok(changed)
    }

    /// Whether today's check-in still needs an attempt, derived from the ledger.
    pub async fn check_in_state(
        &self,
        account_id: &str,
        provider: &Provider,
        now: DateTime<Utc>,
    ) -> Result<CheckInState, DomainError> {
        let id = Self::account(account_id);
        let offset = provider.check_in_day_offset_minutes();
        let quota_day = quota_day_of(now, offset);

        let evidence = self.repository.find_evidence(&id, quota_day).await?;
        let open_attempt = self
            .repository
            .find_open_attempt(&id, quota_day, now - self.policy.attempt_lease)
            .await?;
        let attempts = self.repository.list_attempts_on_day(&id, quota_day).await?;

        Ok(evaluate_check_in_state(
            evidence.as_ref(),
            open_attempt.as_ref(),
            &attempts,
            now,
            next_quota_day_start(quota_day, offset),
            &self.policy,
        ))
    }

    /// Open an attempt so concurrent callers can see a request in flight.
    pub async fn start_attempt(
        &self,
        account_id: &str,
        provider: &Provider,
        quota_day: NaiveDate,
        trigger: CheckInTrigger,
        started_at: DateTime<Utc>,
    ) -> Result<i64, DomainError> {
        let attempt = CheckInAttempt::start(
            Self::account(account_id),
            provider.id().clone(),
            quota_day,
            trigger,
            started_at,
        );
        self.repository.start_attempt(&attempt).await
    }

    pub async fn finish_attempt(
        &self,
        attempt_id: i64,
        completion: &AttemptCompletion,
    ) -> Result<(), DomainError> {
        self.repository.finish_attempt(attempt_id, completion).await
    }

    /// Recompute every account's cached check-in timestamps from the ledger.
    pub async fn refresh_all_check_in_caches(&self) -> Result<usize, DomainError> {
        let ids = self.repository.list_distinct_account_ids().await?;
        for id in &ids {
            self.repository.refresh_check_in_cache(id).await?;
        }
        Ok(ids.len())
    }

    pub async fn get_latest_balance(
        &self,
        account_id: &str,
    ) -> Result<Option<BalanceDto>, DomainError> {
        match self
            .repository
            .find_latest_by_account_id(&Self::account(account_id))
            .await
        {
            Ok(Some(record)) => Ok(Some(BalanceDto {
                current_balance: record.current_balance(),
                total_consumed: record.total_consumed(),
                total_quota: record.total_quota(),
            })),
            Ok(None) => Ok(None),
            Err(e) => {
                warn!(account_id, "Failed to query latest balance history: {}", e);
                Err(e)
            }
        }
    }
}
