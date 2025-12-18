use std::time::Duration;

/// Centralized timeout and duration configuration
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// WAF challenge wait time (default: 8 seconds)
    pub waf_wait: Duration,

    /// Check-in processing wait time after successful check-in (default: 1.5 seconds)
    pub check_in_processing: Duration,

    /// Browser close timeout (default: 5 seconds)
    pub browser_close: Duration,

    /// Browser pool idle timeout (default: 5 minutes)
    pub pool_idle: Duration,

    /// Browser page load timeout (default: 30 seconds)
    pub page_load: Duration,

    /// WAF cookies cache duration (default: 24 hours)
    pub waf_cache_duration: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            waf_wait: Duration::from_secs(8),
            check_in_processing: Duration::from_millis(1500),
            browser_close: Duration::from_secs(5),
            pool_idle: Duration::from_secs(300),
            page_load: Duration::from_secs(30),
            waf_cache_duration: Duration::from_secs(24 * 60 * 60),
        }
    }
}

impl TimeoutConfig {
    /// Create a new timeout configuration with custom values
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder pattern: set WAF wait time
    pub fn with_waf_wait(mut self, duration: Duration) -> Self {
        self.waf_wait = duration;
        self
    }

    /// Builder pattern: set check-in processing time
    pub fn with_check_in_processing(mut self, duration: Duration) -> Self {
        self.check_in_processing = duration;
        self
    }

    /// Builder pattern: set browser close timeout
    pub fn with_browser_close(mut self, duration: Duration) -> Self {
        self.browser_close = duration;
        self
    }

    /// Builder pattern: set pool idle timeout
    pub fn with_pool_idle(mut self, duration: Duration) -> Self {
        self.pool_idle = duration;
        self
    }

    /// Builder pattern: set page load timeout
    pub fn with_page_load(mut self, duration: Duration) -> Self {
        self.page_load = duration;
        self
    }

    /// Builder pattern: set WAF cache duration
    pub fn with_waf_cache_duration(mut self, duration: Duration) -> Self {
        self.waf_cache_duration = duration;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TimeoutConfig::default();
        assert_eq!(config.waf_wait, Duration::from_secs(8));
        assert_eq!(config.check_in_processing, Duration::from_millis(1500));
        assert_eq!(config.browser_close, Duration::from_secs(5));
        assert_eq!(config.pool_idle, Duration::from_secs(300));
    }

    #[test]
    fn test_builder_pattern() {
        let config = TimeoutConfig::new()
            .with_waf_wait(Duration::from_secs(10))
            .with_check_in_processing(Duration::from_secs(2));

        assert_eq!(config.waf_wait, Duration::from_secs(10));
        assert_eq!(config.check_in_processing, Duration::from_secs(2));
    }
}
