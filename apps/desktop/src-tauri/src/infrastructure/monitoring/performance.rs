use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Performance monitor for tracking operation durations
pub struct PerformanceMonitor {
    operation: String,
    start: Instant,
    warn_threshold: Duration,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            start: Instant::now(),
            warn_threshold: Duration::from_millis(1000), // 1 second default
        }
    }

    /// Create a monitor with custom warning threshold
    pub fn with_threshold(operation: impl Into<String>, threshold: Duration) -> Self {
        Self {
            operation: operation.into(),
            start: Instant::now(),
            warn_threshold: threshold,
        }
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Finish monitoring and log the result
    pub fn finish(self) -> Duration {
        let elapsed = self.elapsed();
        
        if elapsed > self.warn_threshold {
            warn!(
                "⚠️  SLOW OPERATION: '{}' took {:.2}ms (threshold: {:.2}ms)",
                self.operation,
                elapsed.as_secs_f64() * 1000.0,
                self.warn_threshold.as_secs_f64() * 1000.0
            );
        } else {
            info!(
                "✓ '{}' completed in {:.2}ms",
                self.operation,
                elapsed.as_secs_f64() * 1000.0
            );
        }
        
        elapsed
    }
}

impl Drop for PerformanceMonitor {
    fn drop(&mut self) {
        let elapsed = self.elapsed();
        
        if elapsed > self.warn_threshold {
            warn!(
                "⚠️  SLOW OPERATION: '{}' took {:.2}ms (threshold: {:.2}ms)",
                self.operation,
                elapsed.as_secs_f64() * 1000.0,
                self.warn_threshold.as_secs_f64() * 1000.0
            );
        } else {
            info!(
                "✓ '{}' completed in {:.2}ms",
                self.operation,
                elapsed.as_secs_f64() * 1000.0
            );
        }
    }
}

/// Macro for easy performance monitoring
#[macro_export]
macro_rules! monitor_performance {
    ($operation:expr) => {
        $crate::infrastructure::monitoring::PerformanceMonitor::new($operation)
    };
    ($operation:expr, $threshold_ms:expr) => {
        $crate::infrastructure::monitoring::PerformanceMonitor::with_threshold(
            $operation,
            std::time::Duration::from_millis($threshold_ms)
        )
    };
}

/// Database query performance metrics
#[derive(Debug, Clone)]
pub struct QueryMetrics {
    pub query_name: String,
    pub duration: Duration,
    pub rows_affected: Option<u64>,
}

impl QueryMetrics {
    pub fn new(query_name: impl Into<String>, duration: Duration) -> Self {
        Self {
            query_name: query_name.into(),
            duration,
            rows_affected: None,
        }
    }

    pub fn with_rows(mut self, rows: u64) -> Self {
        self.rows_affected = Some(rows);
        self
    }

    pub fn log(&self) {
        let duration_ms = self.duration.as_secs_f64() * 1000.0;
        
        if let Some(rows) = self.rows_affected {
            info!(
                "📊 Query '{}': {:.2}ms, {} rows",
                self.query_name, duration_ms, rows
            );
        } else {
            info!(
                "📊 Query '{}': {:.2}ms",
                self.query_name, duration_ms
            );
        }

        // Warn if query is slow
        if duration_ms > 100.0 {
            warn!(
                "🐌 SLOW QUERY: '{}' took {:.2}ms",
                self.query_name, duration_ms
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_performance_monitor_fast_operation() {
        let monitor = PerformanceMonitor::new("test_operation");
        thread::sleep(Duration::from_millis(10));
        let elapsed = monitor.finish();
        
        assert!(elapsed >= Duration::from_millis(10));
        assert!(elapsed < Duration::from_millis(100));
    }

    #[test]
    fn test_performance_monitor_with_threshold() {
        let monitor = PerformanceMonitor::with_threshold(
            "slow_operation",
            Duration::from_millis(5)
        );
        thread::sleep(Duration::from_millis(10));
        let elapsed = monitor.finish();
        
        assert!(elapsed >= Duration::from_millis(10));
    }

    #[test]
    fn test_query_metrics() {
        let metrics = QueryMetrics::new("SELECT * FROM accounts", Duration::from_millis(50))
            .with_rows(10);
        
        assert_eq!(metrics.query_name, "SELECT * FROM accounts");
        assert_eq!(metrics.rows_affected, Some(10));
        
        metrics.log();
    }
}
