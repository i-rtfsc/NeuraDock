use serde::{Deserialize, Serialize};

/// Where a reminder is delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReminderChannel {
    /// Native OS notification from the desktop app.
    App,
    /// Feishu custom-bot webhook.
    Feishu,
}

impl ReminderChannel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Feishu => "feishu",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "app" => Some(Self::App),
            "feishu" => Some(Self::Feishu),
            _ => None,
        }
    }
}

/// A local wall-clock time at which to check for missed check-ins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReminderTime {
    pub hour: u32,
    pub minute: u32,
}

impl ReminderTime {
    pub fn new(hour: u32, minute: u32) -> Option<Self> {
        (hour < 24 && minute < 60).then_some(Self { hour, minute })
    }

    /// Parse `HH:MM`. Returns `None` for anything malformed rather than
    /// defaulting to midnight, so a typo can't silently become a 00:00
    /// reminder.
    pub fn parse(raw: &str) -> Option<Self> {
        let (h, m) = raw.trim().split_once(':')?;
        Self::new(h.trim().parse().ok()?, m.trim().parse().ok()?)
    }

    pub fn to_hh_mm(self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }
}

/// Missed check-in reminder configuration (singleton).
#[derive(Debug, Clone, PartialEq)]
pub struct ReminderSettings {
    enabled: bool,
    times: Vec<ReminderTime>,
    channels: Vec<ReminderChannel>,
    stop_when_all_done: bool,
}

impl Default for ReminderSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            times: vec![
                ReminderTime {
                    hour: 14,
                    minute: 0,
                },
                ReminderTime {
                    hour: 15,
                    minute: 0,
                },
                ReminderTime {
                    hour: 20,
                    minute: 0,
                },
            ],
            channels: vec![ReminderChannel::App, ReminderChannel::Feishu],
            stop_when_all_done: true,
        }
    }
}

impl ReminderSettings {
    pub fn new(
        enabled: bool,
        times: Vec<ReminderTime>,
        channels: Vec<ReminderChannel>,
        stop_when_all_done: bool,
    ) -> Self {
        let mut times = times;
        times.sort();
        times.dedup();
        let mut channels = channels;
        channels.dedup();
        Self {
            enabled,
            times,
            channels,
            stop_when_all_done,
        }
    }

    /// Reconstruct from the stored comma-separated columns.
    ///
    /// Unparseable entries are dropped rather than failing the whole load:
    /// a bad time string should cost that one reminder, not the user's
    /// entire configuration.
    pub fn from_storage(
        enabled: bool,
        times: &str,
        channels: &str,
        stop_when_all_done: bool,
    ) -> Self {
        Self::new(
            enabled,
            times.split(',').filter_map(ReminderTime::parse).collect(),
            channels
                .split(',')
                .filter_map(ReminderChannel::parse)
                .collect(),
            stop_when_all_done,
        )
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn times(&self) -> &[ReminderTime] {
        &self.times
    }

    pub fn channels(&self) -> &[ReminderChannel] {
        &self.channels
    }

    pub fn stop_when_all_done(&self) -> bool {
        self.stop_when_all_done
    }

    pub fn times_to_storage(&self) -> String {
        self.times
            .iter()
            .map(|t| t.to_hh_mm())
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn channels_to_storage(&self) -> String {
        self.channels
            .iter()
            .map(|c| c.as_str())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Whether a reminder is due at this local wall-clock minute.
    ///
    /// Matches to the minute, so the caller must not tick faster than once
    /// per minute or the same time point fires repeatedly.
    pub fn is_due_at(&self, hour: u32, minute: u32) -> bool {
        self.enabled
            && !self.channels.is_empty()
            && self
                .times
                .iter()
                .any(|t| t.hour == hour && t.minute == minute)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_times() {
        assert_eq!(
            ReminderTime::parse("14:00"),
            Some(ReminderTime {
                hour: 14,
                minute: 0
            })
        );
        assert_eq!(
            ReminderTime::parse(" 9:05 "),
            Some(ReminderTime { hour: 9, minute: 5 })
        );
        assert_eq!(
            ReminderTime::parse("23:59"),
            Some(ReminderTime {
                hour: 23,
                minute: 59
            })
        );
    }

    /// Out-of-range and malformed values must be rejected outright. Clamping
    /// or defaulting would turn a typo into a reminder at an hour the user
    /// never asked for -- most likely 00:00.
    #[test]
    fn rejects_invalid_times_instead_of_defaulting() {
        for bad in ["24:00", "12:60", "-1:00", "abc", "12", "", "12:", ":30"] {
            assert_eq!(ReminderTime::parse(bad), None, "should reject {bad:?}");
        }
    }

    #[test]
    fn from_storage_drops_bad_entries_but_keeps_good_ones() {
        let settings =
            ReminderSettings::from_storage(true, "14:00,nonsense,20:00", "app,bogus", true);

        assert_eq!(settings.times().len(), 2);
        assert_eq!(settings.channels(), &[ReminderChannel::App]);
    }

    #[test]
    fn times_are_sorted_and_deduped() {
        let settings =
            ReminderSettings::from_storage(true, "20:00,14:00,20:00", "app,feishu", true);

        let rendered = settings.times_to_storage();
        assert_eq!(rendered, "14:00,20:00");
    }

    #[test]
    fn storage_round_trips() {
        let original =
            ReminderSettings::from_storage(true, "14:00,15:00,20:00", "app,feishu", true);
        let restored = ReminderSettings::from_storage(
            original.enabled(),
            &original.times_to_storage(),
            &original.channels_to_storage(),
            original.stop_when_all_done(),
        );

        assert_eq!(original, restored);
    }

    #[test]
    fn is_due_only_at_configured_minutes() {
        let settings = ReminderSettings::from_storage(true, "14:00,20:30", "app", true);

        assert!(settings.is_due_at(14, 0));
        assert!(settings.is_due_at(20, 30));
        assert!(!settings.is_due_at(14, 1));
        assert!(!settings.is_due_at(20, 0));
        assert!(!settings.is_due_at(15, 0));
    }

    /// Disabling, or removing every delivery channel, must stop reminders.
    /// Otherwise the loop would keep firing into nowhere.
    #[test]
    fn is_never_due_when_disabled_or_channel_less() {
        let disabled = ReminderSettings::from_storage(false, "14:00", "app,feishu", true);
        assert!(!disabled.is_due_at(14, 0));

        let no_channels = ReminderSettings::from_storage(true, "14:00", "", true);
        assert!(!no_channels.is_due_at(14, 0));
    }

    #[test]
    fn default_times_land_after_the_morning_check_in_window() {
        let settings = ReminderSettings::default();
        assert!(!settings.enabled(), "must be opt-in");
        assert!(
            settings.times().iter().all(|t| t.hour >= 12),
            "defaults must not collide with morning auto check-in"
        );
    }
}
