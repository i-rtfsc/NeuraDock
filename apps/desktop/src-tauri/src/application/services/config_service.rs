use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use tracing::info;

/// Log level configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl LogLevel {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => LogLevel::Error,
            2 => LogLevel::Warn,
            3 => LogLevel::Info,
            4 => LogLevel::Debug,
            5 => LogLevel::Trace,
            _ => LogLevel::Info, // Default to Info
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }

    pub fn to_tracing_level(&self) -> tracing::Level {
        match self {
            LogLevel::Error => tracing::Level::ERROR,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Trace => tracing::Level::TRACE,
        }
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

/// Application configuration service
pub struct ConfigService {
    log_level: Arc<AtomicU8>,
}

impl ConfigService {
    pub fn new() -> Self {
        Self {
            log_level: Arc::new(AtomicU8::new(LogLevel::Info as u8)),
        }
    }

    /// Get current log level
    pub fn get_log_level(&self) -> LogLevel {
        let value = self.log_level.load(Ordering::Relaxed);
        LogLevel::from_u8(value)
    }

    /// Set log level
    pub fn set_log_level(&self, level: LogLevel) {
        info!("🔧 Changing log level to: {}", level.as_str());
        self.log_level.store(level as u8, Ordering::Relaxed);
        
        // Update tracing filter dynamically
        // Note: This requires reload_filter feature in tracing-subscriber
        // For now, log level changes will take effect on next app restart
        info!("⚠️  Log level will take effect on next app restart");
    }
}

impl Default for ConfigService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_conversion() {
        assert_eq!(LogLevel::from_u8(1), LogLevel::Error);
        assert_eq!(LogLevel::from_u8(3), LogLevel::Info);
        assert_eq!(LogLevel::from_u8(5), LogLevel::Trace);
        assert_eq!(LogLevel::from_u8(99), LogLevel::Info); // Invalid -> Info
    }

    #[test]
    fn test_log_level_string() {
        assert_eq!(LogLevel::Error.as_str(), "error");
        assert_eq!(LogLevel::Info.as_str(), "info");
        assert_eq!(LogLevel::Trace.as_str(), "trace");
    }

    #[test]
    fn test_config_service() {
        let config = ConfigService::new();
        assert_eq!(config.get_log_level(), LogLevel::Info);

        config.set_log_level(LogLevel::Debug);
        assert_eq!(config.get_log_level(), LogLevel::Debug);

        config.set_log_level(LogLevel::Warn);
        assert_eq!(config.get_log_level(), LogLevel::Warn);
    }
}
