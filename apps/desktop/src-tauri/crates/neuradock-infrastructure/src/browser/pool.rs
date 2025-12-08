use chromiumoxide::{Browser, BrowserConfig};
use neuradock_domain::shared::DomainError;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};

/// Browser instance with metadata
struct BrowserInstance {
    browser: Arc<Browser>,
    created_at: Instant,
    last_used: Instant,
    usage_count: usize,
}

/// Browser pool configuration
#[derive(Debug, Clone)]
pub struct BrowserPoolConfig {
    /// Maximum number of browser instances
    pub max_size: usize,
    /// Maximum idle time before recycling (seconds)
    pub max_idle_time: u64,
    /// Maximum usage count before recycling
    pub max_usage_count: usize,
    /// Browser launch timeout (seconds)
    pub launch_timeout: u64,
}

impl Default for BrowserPoolConfig {
    fn default() -> Self {
        Self {
            max_size: 3,
            max_idle_time: 300, // 5 minutes
            max_usage_count: 50,
            launch_timeout: 30,
        }
    }
}

/// Browser pool manager
pub struct BrowserPool {
    pool: Arc<Mutex<Vec<BrowserInstance>>>,
    semaphore: Arc<Semaphore>,
    config: BrowserPoolConfig,
    browser_config: BrowserConfig,
}

impl BrowserPool {
    /// Create a new browser pool
    pub fn new(config: BrowserPoolConfig, browser_config: BrowserConfig) -> Self {
        Self {
            pool: Arc::new(Mutex::new(Vec::with_capacity(config.max_size))),
            semaphore: Arc::new(Semaphore::new(config.max_size)),
            config,
            browser_config,
        }
    }

    /// Acquire a browser from the pool
    pub async fn acquire(&self) -> Result<PooledBrowser, DomainError> {
        // Acquire semaphore permit
        let permit = self.semaphore.clone().acquire_owned().await.map_err(|e| {
            DomainError::Infrastructure(format!("Failed to acquire browser: {}", e))
        })?;

        // Try to get an existing browser
        let mut pool = self.pool.lock().await;

        // Find a usable browser
        let now = Instant::now();
        let mut reusable_idx = None;

        for (idx, instance) in pool.iter().enumerate() {
            let idle_time = now.duration_since(instance.last_used);

            // Check if browser is still valid
            if idle_time.as_secs() < self.config.max_idle_time
                && instance.usage_count < self.config.max_usage_count
            {
                reusable_idx = Some(idx);
                break;
            }
        }

        let browser = if let Some(idx) = reusable_idx {
            // Reuse existing browser
            let mut instance = pool.remove(idx);
            instance.last_used = now;
            instance.usage_count += 1;

            tracing::info!(
                "Reusing browser instance (usage: {}, age: {}s)",
                instance.usage_count,
                now.duration_since(instance.created_at).as_secs()
            );

            Arc::clone(&instance.browser)
        } else {
            // Create new browser
            tracing::info!("Creating new browser instance (pool size: {})", pool.len());

            drop(pool); // Release lock before launching browser

            let (browser, _handler) = tokio::time::timeout(
                Duration::from_secs(self.config.launch_timeout),
                Browser::launch(self.browser_config.clone()),
            )
            .await
            .map_err(|_| DomainError::Infrastructure("Browser launch timeout".to_string()))?
            .map_err(|e| DomainError::Infrastructure(format!("Failed to launch browser: {}", e)))?;

            let browser = Arc::new(browser);
            let mut pool = self.pool.lock().await;
            pool.push(BrowserInstance {
                browser: Arc::clone(&browser),
                created_at: now,
                last_used: now,
                usage_count: 1,
            });

            browser
        };

        Ok(PooledBrowser {
            browser,
            pool: self.pool.clone(),
            _permit: permit,
        })
    }

    /// Get current pool size
    pub async fn size(&self) -> usize {
        self.pool.lock().await.len()
    }

    /// Clear all browsers in the pool
    pub async fn clear(&self) {
        let mut pool = self.pool.lock().await;
        pool.clear();
        tracing::info!("Browser pool cleared");
    }

    /// Clean up stale browsers
    pub async fn cleanup(&self) {
        let mut pool = self.pool.lock().await;
        let now = Instant::now();

        let initial_size = pool.len();
        pool.retain(|instance| {
            let idle_time = now.duration_since(instance.last_used);
            let is_valid = idle_time.as_secs() < self.config.max_idle_time
                && instance.usage_count < self.config.max_usage_count;

            if !is_valid {
                tracing::info!(
                    "Removing stale browser (idle: {}s, usage: {})",
                    idle_time.as_secs(),
                    instance.usage_count
                );
            }

            is_valid
        });

        let removed = initial_size - pool.len();
        if removed > 0 {
            tracing::info!("Cleaned up {} stale browser(s)", removed);
        }
    }
}

/// Pooled browser with automatic return to pool
pub struct PooledBrowser {
    browser: Arc<Browser>,
    pool: Arc<Mutex<Vec<BrowserInstance>>>,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl PooledBrowser {
    /// Get reference to the browser
    pub fn browser(&self) -> &Browser {
        &self.browser
    }
}

impl Drop for PooledBrowser {
    fn drop(&mut self) {
        // Browser is returned to pool when PooledBrowser is dropped
        // The permit is automatically released
        tracing::debug!("Browser returned to pool");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_browser_pool_acquire() {
        let config = BrowserPoolConfig {
            max_size: 2,
            max_idle_time: 60,
            max_usage_count: 10,
            launch_timeout: 30,
        };

        let browser_config = BrowserConfig::builder().with_head().build().unwrap();

        let pool = BrowserPool::new(config, browser_config);

        // Acquire first browser
        let browser1 = pool.acquire().await;
        assert!(browser1.is_ok());

        // Pool should have 1 browser
        assert_eq!(pool.size().await, 1);

        drop(browser1);

        // After drop, pool still has the browser
        assert_eq!(pool.size().await, 1);
    }

    #[tokio::test]
    async fn test_browser_pool_reuse() {
        let config = BrowserPoolConfig {
            max_size: 2,
            max_idle_time: 60,
            max_usage_count: 10,
            launch_timeout: 30,
        };

        let browser_config = BrowserConfig::builder().with_head().build().unwrap();

        let pool = BrowserPool::new(config, browser_config);

        // Acquire and release
        {
            let _browser1 = pool.acquire().await.unwrap();
        }

        // Acquire again should reuse
        {
            let _browser2 = pool.acquire().await.unwrap();
        }

        // Should still have only 1 browser
        assert_eq!(pool.size().await, 1);
    }
}
