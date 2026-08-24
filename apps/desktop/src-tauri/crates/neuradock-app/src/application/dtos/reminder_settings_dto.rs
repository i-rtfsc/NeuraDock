use serde::{Deserialize, Serialize};
use specta::Type;

use neuradock_domain::reminder_settings::{ReminderChannel, ReminderSettings, ReminderTime};

/// Reminder settings as seen by the frontend.
///
/// Times and channels are exposed as arrays rather than the comma-joined
/// strings used for storage, so the UI never has to parse them.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ReminderSettingsDto {
    pub enabled: bool,
    /// Local wall-clock times as `HH:MM`.
    pub times: Vec<String>,
    /// Any of: `app`, `feishu`.
    pub channels: Vec<String>,
    pub stop_when_all_done: bool,
}

impl From<&ReminderSettings> for ReminderSettingsDto {
    fn from(settings: &ReminderSettings) -> Self {
        Self {
            enabled: settings.enabled(),
            times: settings.times().iter().map(|t| t.to_hh_mm()).collect(),
            channels: settings
                .channels()
                .iter()
                .map(|c| c.as_str().to_string())
                .collect(),
            stop_when_all_done: settings.stop_when_all_done(),
        }
    }
}

/// Input for updating reminder settings.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UpdateReminderSettingsInput {
    pub enabled: bool,
    pub times: Vec<String>,
    pub channels: Vec<String>,
    pub stop_when_all_done: bool,
}

impl UpdateReminderSettingsInput {
    /// Validate and convert to the domain type.
    ///
    /// Unlike the storage path -- which tolerates junk so one bad row can't
    /// lock the user out of their own settings -- input from the UI is
    /// rejected on the first bad entry. Silently dropping a time here would
    /// leave the user believing they had scheduled a reminder that will
    /// never fire.
    pub fn to_domain(&self) -> Result<ReminderSettings, String> {
        let mut times = Vec::with_capacity(self.times.len());
        for raw in &self.times {
            let parsed =
                ReminderTime::parse(raw).ok_or_else(|| format!("Invalid reminder time: {raw}"))?;
            times.push(parsed);
        }

        let mut channels = Vec::with_capacity(self.channels.len());
        for raw in &self.channels {
            let parsed = match raw.trim().to_ascii_lowercase().as_str() {
                "app" => ReminderChannel::App,
                "feishu" => ReminderChannel::Feishu,
                other => return Err(format!("Unknown reminder channel: {other}")),
            };
            channels.push(parsed);
        }

        if self.enabled {
            if times.is_empty() {
                return Err("At least one reminder time is required".to_string());
            }
            if channels.is_empty() {
                return Err("At least one reminder channel is required".to_string());
            }
        }

        Ok(ReminderSettings::new(
            self.enabled,
            times,
            channels,
            self.stop_when_all_done,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_a_malformed_time_rather_than_dropping_it() {
        let input = UpdateReminderSettingsInput {
            enabled: true,
            times: vec!["14:00".into(), "25:00".into()],
            channels: vec!["app".into()],
            stop_when_all_done: true,
        };

        let err = input.to_domain().unwrap_err();
        assert!(
            err.contains("25:00"),
            "error should name the bad value: {err}"
        );
    }

    #[test]
    fn rejects_unknown_channel() {
        let input = UpdateReminderSettingsInput {
            enabled: true,
            times: vec!["14:00".into()],
            channels: vec!["telegram".into()],
            stop_when_all_done: true,
        };

        assert!(input.to_domain().unwrap_err().contains("telegram"));
    }

    /// Enabling with nothing configured would be a reminder that can never
    /// fire, which reads as a broken feature rather than an empty one.
    #[test]
    fn rejects_enabled_with_no_times_or_channels() {
        let no_times = UpdateReminderSettingsInput {
            enabled: true,
            times: vec![],
            channels: vec!["app".into()],
            stop_when_all_done: true,
        };
        assert!(no_times.to_domain().is_err());

        let no_channels = UpdateReminderSettingsInput {
            enabled: true,
            times: vec!["14:00".into()],
            channels: vec![],
            stop_when_all_done: true,
        };
        assert!(no_channels.to_domain().is_err());
    }

    /// A disabled config may legitimately be empty -- that's how the user
    /// turns the feature off without losing anything.
    #[test]
    fn allows_disabled_with_empty_config() {
        let input = UpdateReminderSettingsInput {
            enabled: false,
            times: vec![],
            channels: vec![],
            stop_when_all_done: true,
        };

        assert!(input.to_domain().is_ok());
    }

    #[test]
    fn dto_round_trips_through_domain() {
        let input = UpdateReminderSettingsInput {
            enabled: true,
            times: vec!["20:00".into(), "14:00".into()],
            channels: vec!["app".into(), "feishu".into()],
            stop_when_all_done: true,
        };

        let dto = ReminderSettingsDto::from(&input.to_domain().unwrap());

        // Sorted on the way through the domain type.
        assert_eq!(dto.times, vec!["14:00", "20:00"]);
        assert_eq!(dto.channels, vec!["app", "feishu"]);
    }
}
