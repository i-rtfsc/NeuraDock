//! The day bucket a check-in belongs to.
//!
//! Providers reset their daily quota at a wall-clock time in their own
//! timezone, not at UTC midnight. A check-in recorded at 16:30 UTC belongs to
//! the *next* day for a provider that resets at UTC+8. Storing a UTC calendar
//! day would therefore put early-morning check-ins on the previous day and
//! shift a streak by one, so every day bucket in the ledger, the calendar and
//! the streak is derived from the stored instant plus the provider's offset.
//!
//! Offsets are minutes east of UTC (480 for UTC+8).

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};

/// Providers that reset at UTC+8 unless configured otherwise. These are all
/// CN-hosted services today; a custom provider can override the value.
pub const DEFAULT_DAY_OFFSET_MINUTES: i32 = 480;

/// The quota day an instant belongs to.
pub fn quota_day_of(instant: DateTime<Utc>, offset_minutes: i32) -> NaiveDate {
    (instant + Duration::minutes(offset_minutes as i64)).date_naive()
}

/// The instant a quota day starts (its midnight, expressed in UTC).
pub fn quota_day_start(day: NaiveDate, offset_minutes: i32) -> DateTime<Utc> {
    let local_midnight = day.and_time(NaiveTime::MIN).and_utc();
    local_midnight - Duration::minutes(offset_minutes as i64)
}

/// The instant the day after `day` starts -- when backoff stops applying.
pub fn next_quota_day_start(day: NaiveDate, offset_minutes: i32) -> DateTime<Utc> {
    quota_day_start(day.succ_opt().unwrap_or(day), offset_minutes)
}

/// Today's quota day for a provider.
pub fn today(offset_minutes: i32, now: DateTime<Utc>) -> NaiveDate {
    quota_day_of(now, offset_minutes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instant(value: &str) -> DateTime<Utc> {
        value.parse().expect("valid RFC3339 instant")
    }

    #[test]
    fn a_utc_instant_after_local_midnight_belongs_to_the_next_local_day() {
        // 16:30 UTC is 00:30 the next day at UTC+8.
        assert_eq!(
            quota_day_of(instant("2026-09-10T16:30:00Z"), 480),
            NaiveDate::from_ymd_opt(2026, 9, 11).unwrap()
        );
        // The same instant is still the previous day in UTC.
        assert_eq!(
            quota_day_of(instant("2026-09-10T16:30:00Z"), 0),
            NaiveDate::from_ymd_opt(2026, 9, 10).unwrap()
        );
    }

    #[test]
    fn midday_instants_agree_between_utc_and_utc_plus_eight() {
        let noon = instant("2026-09-10T04:00:00Z"); // 12:00 local
        assert_eq!(quota_day_of(noon, 480), quota_day_of(noon, 0));
    }

    #[test]
    fn day_bounds_are_the_inverse_of_the_bucket() {
        let day = NaiveDate::from_ymd_opt(2026, 9, 11).unwrap();
        let start = quota_day_start(day, 480);
        let just_before = start - Duration::seconds(1);
        let just_after = start + Duration::seconds(1);

        assert_eq!(quota_day_of(start, 480), day);
        assert_eq!(quota_day_of(just_after, 480), day);
        assert_eq!(
            quota_day_of(just_before, 480),
            NaiveDate::from_ymd_opt(2026, 9, 10).unwrap()
        );
    }

    #[test]
    fn next_day_start_is_one_day_after_this_one() {
        let day = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        assert_eq!(
            next_quota_day_start(day, 480),
            quota_day_start(NaiveDate::from_ymd_opt(2027, 1, 1).unwrap(), 480)
        );
    }

    #[test]
    fn west_of_utc_offsets_round_trip() {
        let day = NaiveDate::from_ymd_opt(2026, 9, 11).unwrap();
        let start = quota_day_start(day, -300);
        assert_eq!(quota_day_of(start, -300), day);
    }
}
