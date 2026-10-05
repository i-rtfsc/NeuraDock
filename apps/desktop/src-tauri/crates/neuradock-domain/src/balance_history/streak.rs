//! Streak arithmetic over the evidence ledger.
//!
//! Kept as a pure function so it can be tested directly: the previous version
//! lived in the query layer with no tests and asked the wrong question. It
//! walked the rows and only reset the counter when it *observed* a missing day,
//! so a run that ended a week ago was still reported as current whenever no
//! later row existed. A streak is only current if it reaches today or
//! yesterday; everything else is history.

use chrono::NaiveDate;

use super::{BalanceHistoryDailySummary, CheckInConfidence};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreakSummary {
    /// Run of consecutive checked-in days ending no earlier than yesterday.
    pub current: u32,
    pub longest: u32,
    pub total_days: u32,
    pub last_check_in_date: Option<NaiveDate>,
    /// How many of `total_days` were inferred from a quota increase rather than
    /// confirmed, so the UI can say so instead of presenting them as facts.
    pub inferred_days: u32,
}

/// Compute the streak summary for one account.
///
/// `days` may contain any mix of days (evidence days and plain snapshot days);
/// only rows carrying evidence count. `today` is the account's current quota
/// day, which is what makes "current" meaningful.
pub fn compute_streak(days: &[BalanceHistoryDailySummary], today: NaiveDate) -> StreakSummary {
    let mut current: u32 = 0;
    let mut longest: u32 = 0;
    let mut total_days: u32 = 0;
    let mut inferred_days: u32 = 0;
    let mut last_check_in_date: Option<NaiveDate> = None;

    for day in days.iter().filter(|day| day.is_checked_in()) {
        let date = day.quota_day();
        total_days += 1;
        if day.check_in_confidence() == Some(CheckInConfidence::Inferred) {
            inferred_days += 1;
        }

        current = match last_check_in_date {
            Some(previous) if (date - previous).num_days() == 1 => current + 1,
            _ => 1,
        };
        longest = longest.max(current);
        last_check_in_date = Some(date);
    }

    // A run is only "current" while today or yesterday is part of it.
    if let Some(last) = last_check_in_date {
        if (today - last).num_days() > 1 {
            current = 0;
        }
    }

    StreakSummary {
        current,
        longest,
        total_days,
        last_check_in_date,
        inferred_days,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::balance_history::CheckInSource;

    fn day(value: &str) -> NaiveDate {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
    }

    fn summary(quota_day: &str, source: CheckInSource) -> BalanceHistoryDailySummary {
        BalanceHistoryDailySummary::restore(day(quota_day), 0.0, 0.0, 0.0)
            .with_check_in(Some(25.0), Some(source))
    }

    fn snapshot_day(quota_day: &str) -> BalanceHistoryDailySummary {
        BalanceHistoryDailySummary::restore(day(quota_day), 100.0, 100.0, 0.0)
    }

    #[test]
    fn an_empty_ledger_has_no_streak() {
        let result = compute_streak(&[], day("2026-01-10"));
        assert_eq!(result.current, 0);
        assert_eq!(result.longest, 0);
        assert_eq!(result.total_days, 0);
        assert_eq!(result.last_check_in_date, None);
    }

    #[test]
    fn consecutive_days_accumulate_and_the_longest_is_remembered() {
        let days = [
            summary("2026-01-01", CheckInSource::ProviderResponse),
            summary("2026-01-02", CheckInSource::ProviderResponse),
            summary("2026-01-03", CheckInSource::ProviderResponse),
            summary("2026-01-05", CheckInSource::ProviderResponse),
            summary("2026-01-06", CheckInSource::ProviderResponse),
        ];
        // Today is the 6th, so the second run is current.
        let result = compute_streak(&days, day("2026-01-06"));
        assert_eq!(result.current, 2);
        assert_eq!(result.longest, 3);
        assert_eq!(result.total_days, 5);
        assert_eq!(result.last_check_in_date, Some(day("2026-01-06")));
    }

    #[test]
    fn a_run_that_ended_yesterday_is_still_current() {
        let days = [
            summary("2026-01-08", CheckInSource::ProviderResponse),
            summary("2026-01-09", CheckInSource::ProviderResponse),
        ];
        let result = compute_streak(&days, day("2026-01-10"));
        assert_eq!(result.current, 2, "today can still be checked in");
    }

    #[test]
    fn a_run_that_ended_two_days_ago_is_not_current() {
        // This is the regression: with no later rows the old algorithm kept
        // reporting the stale run as current forever.
        let days = [
            summary("2026-01-01", CheckInSource::ProviderResponse),
            summary("2026-01-02", CheckInSource::ProviderResponse),
            summary("2026-01-03", CheckInSource::ProviderResponse),
        ];
        let result = compute_streak(&days, day("2026-01-10"));
        assert_eq!(result.current, 0);
        assert_eq!(result.longest, 3);
        assert_eq!(result.total_days, 3);
    }

    #[test]
    fn snapshot_only_days_break_a_run_without_counting_as_evidence() {
        let days = [
            summary("2026-01-01", CheckInSource::ProviderResponse),
            snapshot_day("2026-01-02"),
            summary("2026-01-03", CheckInSource::ProviderResponse),
        ];
        let result = compute_streak(&days, day("2026-01-03"));
        assert_eq!(result.current, 1, "the gap breaks the run");
        assert_eq!(result.longest, 1);
        assert_eq!(result.total_days, 2);
        assert_eq!(result.inferred_days, 0);
    }

    #[test]
    fn inferred_days_are_counted_and_reported() {
        let days = [
            summary("2026-01-01", CheckInSource::ProviderResponse),
            summary("2026-01-02", CheckInSource::BalanceDelta),
            summary("2026-01-03", CheckInSource::LegacyEstimate),
            summary("2026-01-04", CheckInSource::Manual),
        ];
        let result = compute_streak(&days, day("2026-01-04"));
        assert_eq!(result.current, 4);
        assert_eq!(result.total_days, 4);
        assert_eq!(result.inferred_days, 2, "balance delta and legacy estimate");
    }

    #[test]
    fn a_run_spanning_a_month_boundary_stays_consecutive() {
        let days = [
            summary("2025-12-31", CheckInSource::ProviderResponse),
            summary("2026-01-01", CheckInSource::ProviderResponse),
        ];
        let result = compute_streak(&days, day("2026-01-01"));
        assert_eq!(result.current, 2);
        assert_eq!(result.longest, 2);
    }

    #[test]
    fn an_unsorted_ledger_still_produces_the_right_longest_run() {
        // The repository returns days ordered, but the function must not depend
        // on it silently: an out-of-order pair simply breaks the run.
        let days = [
            summary("2026-01-03", CheckInSource::ProviderResponse),
            summary("2026-01-01", CheckInSource::ProviderResponse),
            summary("2026-01-02", CheckInSource::ProviderResponse),
        ];
        let result = compute_streak(&days, day("2026-01-03"));
        assert_eq!(result.total_days, 3);
        assert_eq!(result.longest, 2);
    }
}
