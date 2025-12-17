use std::time::Duration;

/// Configuration for various timeout durations across the application
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// WAF bypass wait time for page load
    pub waf_wait: Duration,
    
    /// Check-in processing wait time
    pub check_in_processing: Duration,
    
    /// Browser close timeout
    pub browser_close: Duration,
    
    /// Browser launch timeout
    pub browser_launch: Duration,
    
    /// Idle time before browser pool cleanup
    pub pool_idle: Duration,
    
    /// HTTP request timeout
    pub http_request: Duration,
    
    /// Database query timeout
    pub db_query: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            waf_wait: Duration::from_secs(8),
            check_in_processing: Duration::from_millis(1500),
            browser_close: Duration::from_secs(5),
            browser_launch: Duration::from_secs(30),
            pool_idle: Duration::from_secs(300),
            http_request: Duration::from_secs(30),
            db_query: Duration::from_secs(10),
        }
    }
}

impl TimeoutConfig {
    /// Create a new timeout configuration with custom values
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get the global timeout configuration
    pub fn global() -> &'static Self {
        &GLOBAL_TIMEOUT_CONFIG
    }
}

/// Global timeout configuration instance
static GLOBAL_TIMEOUT_CONFIG: TimeoutConfig = TimeoutConfig {
    waf_wait: Duration::from_secs(8),
    check_in_processing: Duration::from_millis(1500),
    browser_close: Duration::from_secs(5),
    browser_launch: Duration::from_secs(30),
    pool_idle: Duration::from_secs(300),
    http_request: Duration::from_secs(30),
    db_query: Duration::from_secs(10),
};
