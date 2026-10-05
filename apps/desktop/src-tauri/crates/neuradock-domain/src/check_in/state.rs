//! Whether today's check-in still needs an attempt.
//!
//! This replaces the old "has it been at least 24h minus a grace period?"
//! heuristic. That question had no correct answer: it approximated "is today
//! already done" with elapsed time, needed a 15 minute fudge factor to avoid
//! skipping days, treated a plain balance refresh as a check-in (the cached
//! timestamp it read came from the balance snapshot), and did nothing at all
//! when the configured interval was 0 -- which the shipped default was.
//!
//! Here the question is answered from facts: evidence for the quota day, an
//! open attempt (single-flight), and the tail of today's attempts (backoff).

use chrono::{DateTime, Duration, Utc};

use crate::balance_history::{CheckInAttempt, CheckInConfidence, CheckInOutcome, CheckInRecord};

/// What the caller should do about today's check-in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckInState {
    /// The quota day is already satisfied; sending another request would
    /// duplicate a non-idempotent provider call.
    Done { confidence: CheckInConfidence },
    /// An attempt is open (or was started within the lease window).
    InFlight { attempt_id: Option<i64> },
    /// Wait until `until` before trying again today.
    Backoff {
        until: DateTime<Utc>,
        retryable_failures: u32,
    },
    /// Nothing stands in the way of an attempt.
    Due,
}

impl CheckInState {
    pub fn is_due(&self) -> bool {
        matches!(self, Self::Due)
    }

    pub fn is_done(&self) -> bool {
        matches!(self, Self::Done { .. })
    }

    pub fn is_in_flight(&self) -> bool {
        matches!(self, Self::InFlight { .. })
    }
}

/// Retry pacing. Deliberately not provider-configurable yet: the numbers are a
/// safety throttle, and the *primary* guard (`Done`) is a fact, not a timer.
#[derive(Debug, Clone)]
pub struct CheckInPolicy {
    /// Base delay before retrying a retryable failure.
    pub min_retry_interval: Duration,
    /// How long an attempt with no completion blocks new attempts. Covers a
    /// crash between "started" and "finished" without blocking the day.
    pub attempt_lease: Duration,
    /// Ceiling for the exponential backoff.
    pub max_backoff: Duration,
}

impl Default for CheckInPolicy {
    fn default() -> Self {
        Self {
            min_retry_interval: Duration::minutes(30),
            attempt_lease: Duration::minutes(10),
            max_backoff: Duration::hours(6),
        }
    }
}

impl CheckInPolicy {
    /// Pacing for an account, honouring its configured minimum interval.
    ///
    /// The interval throttles repeated *attempts*; it is not a guard. Whether
    /// today is already done is decided by the ledger, which is the whole point
    /// of this model: elapsed time could never answer that question. Zero keeps
    /// the built-in default, which is what the legacy column's default meant.
    pub fn from_account_min_interval(hours: u8) -> Self {
        let base = Self::default();
        if hours == 0 {
            return base;
        }

        let configured = Duration::hours(i64::from(hours));
        Self {
            min_retry_interval: configured,
            // The ceiling may not sit below the interval the user asked for, or
            // the throttle would silently be shortened.
            max_backoff: configured.max(base.max_backoff),
            ..base
        }
    }

    /// Exponential backoff for N consecutive retryable failures in one day:
    /// 30m, 1h, 2h, 4h, then the 6h ceiling.
    pub fn backoff_for(&self, retryable_failures: u32) -> Duration {
        if retryable_failures == 0 {
            return Duration::zero();
        }
        let doublings = (retryable_failures - 1).min(16);
        let step = self
            .min_retry_interval
            .checked_mul(1i32 << doublings)
            .unwrap_or(self.max_backoff);
        step.min(self.max_backoff)
    }
}

/// Evaluate today's check-in state.
///
/// `attempts_today` is expected oldest-first; only the most recent entries
/// matter. `next_day_start` caps the backoff so a bad day never leaks into the
/// next one.
pub fn evaluate_check_in_state(
    evidence: Option<&CheckInRecord>,
    open_attempt: Option<&CheckInAttempt>,
    attempts_today: &[CheckInAttempt],
    now: DateTime<Utc>,
    next_day_start: DateTime<Utc>,
    policy: &CheckInPolicy,
) -> CheckInState {
    if let Some(evidence) = evidence {
        return CheckInState::Done {
            confidence: evidence.confidence(),
        };
    }

    if let Some(attempt) = open_attempt {
        return CheckInState::InFlight {
            attempt_id: attempt.id(),
        };
    }

    let Some(last) = attempts_today.last() else {
        return CheckInState::Due;
    };

    // A permanent failure means another request today cannot succeed (the
    // provider answered, and the answer was no). Wait for the next quota day
    // instead of hammering.
    if last.outcome() == Some(CheckInOutcome::FailedPermanent) {
        return CheckInState::Backoff {
            until: next_day_start,
            retryable_failures: 0,
        };
    }

    let mut retryable_failures = 0u32;
    let mut last_failure_at: Option<DateTime<Utc>> = None;
    for attempt in attempts_today.iter().rev() {
        match attempt.outcome() {
            Some(outcome) if outcome.backs_off() => {
                retryable_failures += 1;
                if last_failure_at.is_none() {
                    last_failure_at = Some(
                        attempt
                            .finished_at()
                            .unwrap_or_else(|| attempt.started_at()),
                    );
                }
            }
            _ => break,
        }
    }

    match last_failure_at {
        Some(failed_at) => {
            let until = (failed_at + policy.backoff_for(retryable_failures)).min(next_day_start);
            if now < until {
                CheckInState::Backoff {
                    until,
                    retryable_failures,
                }
            } else {
                CheckInState::Due
            }
        }
        None => CheckInState::Due,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::balance_history::{
        AttemptCompletion, CheckInAttempt, CheckInOutcome, CheckInSource, CheckInTrigger,
    };
    use crate::shared::{AccountId, ProviderId};
    use chrono::NaiveDate;

    fn now() -> DateTime<Utc> {
        "2026-09-11T04:00:00Z".parse().unwrap()
    }

    fn day() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 11).unwrap()
    }

    fn next_day() -> DateTime<Utc> {
        "2026-09-11T16:00:00Z".parse().unwrap()
    }

    fn account() -> AccountId {
        AccountId::from_string("acc")
    }

    fn provider() -> ProviderId {
        ProviderId::from_string("prov")
    }

    fn failed_at(at: DateTime<Utc>, outcome: CheckInOutcome) -> CheckInAttempt {
        let mut attempt = CheckInAttempt::start(
            account(),
            provider(),
            day(),
            CheckInTrigger::Scheduler,
            at - Duration::seconds(5),
        );
        attempt.finish(AttemptCompletion {
            outcome,
            finished_at: at,
            error_code: None,
            http_status: None,
            reward: None,
            source: None,
        });
        attempt
    }

    fn evidence(confidence_source: CheckInSource) -> CheckInRecord {
        CheckInRecord::new(account(), day(), now(), Some(25.0), confidence_source).unwrap()
    }

    #[test]
    fn evidence_wins_over_everything_else() {
        let open =
            CheckInAttempt::start(account(), provider(), day(), CheckInTrigger::Manual, now());
        let attempts = vec![failed_at(now(), CheckInOutcome::FailedRetryable)];
        let state = evaluate_check_in_state(
            Some(&evidence(CheckInSource::ProviderResponse)),
            Some(&open),
            &attempts,
            now(),
            next_day(),
            &CheckInPolicy::default(),
        );
        assert_eq!(
            state,
            CheckInState::Done {
                confidence: CheckInConfidence::Confirmed
            }
        );
    }

    #[test]
    fn an_open_attempt_is_single_flight() {
        let open =
            CheckInAttempt::start(account(), provider(), day(), CheckInTrigger::Batch, now());
        let state = evaluate_check_in_state(
            None,
            Some(&open),
            &[],
            now(),
            next_day(),
            &CheckInPolicy::default(),
        );
        assert!(state.is_in_flight());
    }

    #[test]
    fn a_clean_day_is_due() {
        let state = evaluate_check_in_state(
            None,
            None,
            &[],
            now(),
            next_day(),
            &CheckInPolicy::default(),
        );
        assert_eq!(state, CheckInState::Due);
    }

    #[test]
    fn retryable_failures_escalate_and_are_capped() {
        let policy = CheckInPolicy::default();
        assert_eq!(policy.backoff_for(1), Duration::minutes(30));
        assert_eq!(policy.backoff_for(2), Duration::hours(1));
        assert_eq!(policy.backoff_for(3), Duration::hours(2));
        assert_eq!(policy.backoff_for(4), Duration::hours(4));
        assert_eq!(policy.backoff_for(5), Duration::hours(6));
        assert_eq!(policy.backoff_for(99), Duration::hours(6));
    }

    #[test]
    fn a_zero_interval_keeps_the_built_in_throttle() {
        let policy = CheckInPolicy::from_account_min_interval(0);
        assert_eq!(policy.min_retry_interval, Duration::minutes(30));
        assert_eq!(policy.max_backoff, Duration::hours(6));
    }

    #[test]
    fn a_configured_interval_becomes_the_retry_throttle() {
        let policy = CheckInPolicy::from_account_min_interval(24);
        assert_eq!(policy.min_retry_interval, Duration::hours(24));
        assert_eq!(
            policy.max_backoff,
            Duration::hours(24),
            "the ceiling never drops below the interval the user asked for"
        );
        assert_eq!(policy.backoff_for(1), Duration::hours(24));
        assert_eq!(policy.backoff_for(5), Duration::hours(24));
    }

    #[test]
    fn a_recent_failure_holds_the_next_attempt_back() {
        let policy = CheckInPolicy::default();
        let attempts = vec![failed_at(
            now() - Duration::minutes(10),
            CheckInOutcome::FailedRetryable,
        )];
        let state = evaluate_check_in_state(None, None, &attempts, now(), next_day(), &policy);
        assert_eq!(
            state,
            CheckInState::Backoff {
                until: now() + Duration::minutes(20),
                retryable_failures: 1,
            }
        );
    }

    #[test]
    fn the_wait_expires_and_the_day_becomes_due_again() {
        let policy = CheckInPolicy::default();
        let attempts = vec![failed_at(
            now() - Duration::hours(2),
            CheckInOutcome::FailedRetryable,
        )];
        let state = evaluate_check_in_state(None, None, &attempts, now(), next_day(), &policy);
        assert_eq!(state, CheckInState::Due);
    }

    #[test]
    fn backoff_never_leaks_into_the_next_quota_day() {
        let policy = CheckInPolicy::default();
        // Mid-day the backoff is not clamped.
        let attempts = vec![failed_at(
            now() - Duration::minutes(1),
            CheckInOutcome::WafBlocked,
        )];
        assert_eq!(
            evaluate_check_in_state(None, None, &attempts, now(), next_day(), &policy),
            CheckInState::Backoff {
                until: now() + Duration::minutes(29),
                retryable_failures: 1,
            }
        );

        // Late in the day a 30 minute backoff would cross the boundary, so it
        // is clamped to the start of the next quota day.
        let late: DateTime<Utc> = "2026-09-11T15:50:00Z".parse().unwrap();
        let attempts = vec![failed_at(
            late - Duration::minutes(1),
            CheckInOutcome::WafBlocked,
        )];
        let state = evaluate_check_in_state(None, None, &attempts, late, next_day(), &policy);
        match state {
            CheckInState::Backoff { until, .. } => assert_eq!(until, next_day()),
            other => panic!("expected a capped backoff, got {other:?}"),
        }
    }

    #[test]
    fn a_permanent_failure_ends_the_day_without_more_requests() {
        let policy = CheckInPolicy::default();
        let attempts = vec![failed_at(now(), CheckInOutcome::FailedPermanent)];
        let state = evaluate_check_in_state(None, None, &attempts, now(), next_day(), &policy);
        assert_eq!(
            state,
            CheckInState::Backoff {
                until: next_day(),
                retryable_failures: 0,
            }
        );
    }

    #[test]
    fn a_non_retryable_outcome_breaks_the_backoff_chain() {
        let policy = CheckInPolicy::default();
        // The most recent attempt was a guard skip: it must not extend the
        // backoff of the older failure, or a skipped tick would freeze the day.
        let attempts = vec![
            failed_at(
                now() - Duration::minutes(5),
                CheckInOutcome::FailedRetryable,
            ),
            failed_at(now(), CheckInOutcome::SkippedGuard),
        ];
        let state = evaluate_check_in_state(None, None, &attempts, now(), next_day(), &policy);
        assert_eq!(state, CheckInState::Due);
    }
}
