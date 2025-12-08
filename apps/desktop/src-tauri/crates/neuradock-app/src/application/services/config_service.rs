use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
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

/// Persistent configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppConfig {
    log_level: LogLevel,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            log_level: LogLevel::Info,
        }
    }
}

/// Application configuration service
pub struct ConfigService {
    log_level: Arc<AtomicU8>,
    config_path: PathBuf,
}

impl ConfigService {
    /// Create a new ConfigService with persistence
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        // Get app config directory
        let config_dir = app_handle
            .path()
            .app_config_dir()
            .map_err(|e| anyhow::anyhow!("Failed to get config dir: {}", e))?;

        // Ensure config directory exists
        std::fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("app_config.json");

        // Load existing config or create default
        let config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            serde_json::from_str::<AppConfig>(&content).unwrap_or_default()
        } else {
            AppConfig::default()
        };

        info!("📁 Config loaded from: {:?}", config_path);
        info!("🔧 Initial log level: {}", config.log_level.as_str());

        Ok(Self {
            log_level: Arc::new(AtomicU8::new(config.log_level as u8)),
            config_path,
        })
    }

    /// Get current log level
    pub fn get_log_level(&self) -> LogLevel {
        let value = self.log_level.load(Ordering::Relaxed);
        LogLevel::from_u8(value)
    }

    /// Set log level and persist to disk
    pub fn set_log_level(&self, level: LogLevel) -> Result<()> {
        info!("🔧 Changing log level to: {}", level.as_str());
        self.log_level.store(level as u8, Ordering::Relaxed);

        // Persist to disk
        let config = AppConfig { log_level: level };

        let content = serde_json::to_string_pretty(&config)?;
        std::fs::write(&self.config_path, content)?;

        info!("💾 Log level saved to: {:?}", self.config_path);
        info!("⚠️  Log level will take effect on next app restart");

        Ok(())
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
}
