use anyhow::Result;
use chromiumoxide::browser::{Browser, BrowserConfig};
use futures::StreamExt;
use log::{info, warn};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::task::JoinHandle;
use crate::config::TimeoutConfig;

// Import log masking utilities
use crate::logging::log_utils::mask_sensitive;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36";
const REQUIRED_WAF_COOKIES: &[&str] = &["acw_tc", "cdn_sec_tc", "acw_sc__v2"];

/// Helper to clean up browser resources with timeout
async fn cleanup_browser(
    mut browser: Browser,
    handler_task: JoinHandle<()>,
    temp_dir: PathBuf,
    account_name: &str,
) {
    let config = TimeoutConfig::global();

    // Abort the handler task first
    handler_task.abort();

    // Try to close browser with timeout
    match tokio::time::timeout(config.browser_close, browser.close()).await {
        Ok(Ok(_)) => {
            info!("[{}] Browser closed successfully", account_name);
        }
        Ok(Err(e)) => {
            warn!("[{}] Failed to close browser: {}, will force cleanup", account_name, e);
        }
        Err(_) => {
            warn!("[{}] Browser close timed out, continuing with cleanup", account_name);
        }
    }

    // Give Chrome a moment to fully exit
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Try to clean up temp directory
    let cleanup_result = std::fs::remove_dir_all(&temp_dir);

    if let Err(e) = cleanup_result {
        warn!(
            "[{}] Failed to clean up temp directory on first attempt: {}. Retrying after force kill...",
            account_name, e
        );

        // If cleanup failed, Chrome might still be running - try to force kill it
        force_kill_chrome_processes(&temp_dir, account_name).await;

        // Wait a bit more and retry cleanup
        tokio::time::sleep(Duration::from_secs(2)).await;

        if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
            warn!(
                "[{}] Failed to clean up temp directory even after force kill: {}",
                account_name, e
            );
        } else {
            info!("[{}] Cleaned up temp profile directory after force kill", account_name);
        }
    } else {
        info!("[{}] Cleaned up temp profile directory", account_name);
    }
}

/// Force kill Chrome processes that might be using the temp directory
async fn force_kill_chrome_processes(temp_dir: &PathBuf, account_name: &str) {
    #[cfg(unix)]
    {
        use std::process::Command;

        // Get the temp directory path as a string for matching
        let temp_dir_str = temp_dir.to_string_lossy();

        // Find Chrome processes using this user-data-dir
        // This uses lsof to find processes with open files in the temp directory
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "lsof +D '{}' 2>/dev/null | grep Chrome | awk '{{print $2}}' | sort -u",
                temp_dir_str
            ))
            .output();

        if let Ok(output) = output {
            let pids = String::from_utf8_lossy(&output.stdout);
            for pid in pids.lines() {
                if let Ok(pid_num) = pid.trim().parse::<i32>() {
                    warn!(
                        "[{}] Force killing Chrome process with PID: {}",
                        account_name, pid_num
                    );

                    // Send SIGKILL to force terminate
                    let _ = Command::new("kill")
                        .arg("-9")
                        .arg(pid.trim())
                        .output();
                }
            }
        }
    }

    #[cfg(windows)]
    {
        use std::process::Command;

        let temp_dir_str = temp_dir.to_string_lossy();

        // Use tasklist and findstr to find Chrome processes
        // Note: This is a best-effort approach on Windows
        let _ = Command::new("taskkill")
            .args(&["/F", "/IM", "chrome.exe"])
            .output();

        warn!(
            "[{}] Attempted to kill Chrome processes (Windows)",
            account_name
        );
    }
}

/// Find available Chromium-based browser on the system
fn find_browser() -> Option<PathBuf> {
    let browser_paths = vec![
        // macOS
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        // Linux
        "/usr/bin/google-chrome",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/bin/brave-browser",
        "/usr/bin/microsoft-edge",
        "/snap/bin/chromium",
        // Common alternative paths
        "/opt/google/chrome/chrome",
        "/opt/chromium/chromium",
    ];

    // Check standard paths first
    for path in browser_paths {
        let browser_path = PathBuf::from(path);
        if browser_path.exists() {
            return Some(browser_path);
        }
    }

    // Windows-specific detection
    #[cfg(target_os = "windows")]
    {
        // Check Windows registry and common installation paths
        let windows_paths: Vec<String> = vec![
            // Chrome (Program Files)
            r"C:\Program Files\Google\Chrome\Application\chrome.exe".to_string(),
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe".to_string(),
            // Chrome (Local AppData)
            format!(
                r"{}\Google\Chrome\Application\chrome.exe",
                std::env::var("LOCALAPPDATA").unwrap_or_default()
            ),
            // Chromium
            r"C:\Program Files\Chromium\Application\chrome.exe".to_string(),
            r"C:\Program Files (x86)\Chromium\Application\chrome.exe".to_string(),
            // Brave
            r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe".to_string(),
            r"C:\Program Files (x86)\BraveSoftware\Brave-Browser\Application\brave.exe".to_string(),
            format!(
                r"{}\BraveSoftware\Brave-Browser\Application\brave.exe",
                std::env::var("LOCALAPPDATA").unwrap_or_default()
            ),
            // Microsoft Edge
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe".to_string(),
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe".to_string(),
        ];

        for path_str in windows_paths {
            let browser_path = PathBuf::from(&path_str);
            if browser_path.exists() {
                return Some(browser_path);
            }
        }

        // Try to get Chrome path from registry
        if let Ok(chrome_path) = get_chrome_from_registry() {
            if chrome_path.exists() {
                return Some(chrome_path);
            }
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn get_chrome_from_registry() -> Result<PathBuf> {
    use std::process::Command;

    // Try to get Chrome path from Windows registry
    let output = Command::new("reg")
        .args(&[
            "query",
            r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\chrome.exe",
            "/ve",
        ])
        .output();

    if let Ok(output) = output {
        let output_str = String::from_utf8_lossy(&output.stdout);
        // Parse the registry output to extract the path
        for line in output_str.lines() {
            if line.contains("REG_SZ") {
                if let Some(path) = line.split("REG_SZ").nth(1) {
                    let path = path.trim();
                    return Ok(PathBuf::from(path));
                }
            }
        }
    }

    Err(anyhow::anyhow!("Chrome not found in registry"))
}

pub struct WafBypassService {
    headless: bool,
}

impl WafBypassService {
    pub fn new(headless: bool) -> Self {
        Self { headless }
    }

    /// Get WAF cookies using chromiumoxide (pure Rust)
    pub async fn get_waf_cookies(
        &self,
        login_url: &str,
        account_name: &str,
    ) -> Result<HashMap<String, String>> {
        const MAX_RETRIES: u32 = 2;
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                info!(
                    "[{}] Retrying WAF cookie fetch (attempt {}/{})",
                    account_name,
                    attempt + 1,
                    MAX_RETRIES
                );
                tokio::time::sleep(Duration::from_secs(2)).await;
            }

            match self.get_waf_cookies_once(login_url, account_name).await {
                Ok(cookies) => return Ok(cookies),
                Err(e) => {
                    warn!(
                        "[{}] WAF cookie fetch attempt {} failed: {}",
                        account_name,
                        attempt + 1,
                        e
                    );
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!("Failed to get WAF cookies after {} attempts", MAX_RETRIES)
        }))
    }

    /// Internal method to get WAF cookies once
    async fn get_waf_cookies_once(
        &self,
        login_url: &str,
        account_name: &str,
    ) -> Result<HashMap<String, String>> {
        info!(
            "[{}] Starting browser to get WAF cookies (chromiumoxide)...",
            account_name
        );

        // 1. Launch browser with proper configuration
        let (browser, handler_task, temp_dir) = self
            .launch_browser_with_config(account_name)
            .await?;

        // 2. Navigate to page and extract cookies
        let (browser, waf_cookies_result) = self
            .navigate_and_extract_cookies(
                browser,
                login_url,
                account_name,
            )
            .await;

        // 3. Clean up browser resources (always execute even if error)
        cleanup_browser(browser, handler_task, temp_dir, account_name).await;

        // 4. Return result
        let waf_cookies = waf_cookies_result?;

        // Check if we got any cookies
        if waf_cookies.is_empty() {
            let err_msg = format!(
                "No WAF cookies obtained. Expected cookies: {:?}. This might indicate that the page didn't load properly or WAF protection has changed.",
                REQUIRED_WAF_COOKIES
            );
            warn!("[{}] {}", account_name, err_msg);
            anyhow::bail!(err_msg);
        }

        info!(
            "[{}] ✓ Successfully got {} WAF cookies",
            account_name,
            waf_cookies.len()
        );

        Ok(waf_cookies)
    }

    /// Launch browser with proper configuration
    /// Returns (browser, handler_task, temp_dir)
    async fn launch_browser_with_config(
        &self,
        account_name: &str,
    ) -> Result<(Browser, JoinHandle<()>, PathBuf)> {
        // Use unique temporary directory for each session to avoid lock conflicts
        let temp_dir = std::env::temp_dir().join(format!("chromiumoxide-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create temp directory: {}", e))?;

        info!(
            "[{}] Using temp profile directory: {:?}",
            account_name, temp_dir
        );

        // Find available browser
        let browser_path = find_browser().ok_or_else(|| {
            let err_msg = "No Chromium-based browser found. Please install one of: Google Chrome, Chromium, Brave, or Microsoft Edge";
            log::error!("[{}] {}", account_name, err_msg);
            anyhow::anyhow!(err_msg)
        })?;

        info!("[{}] Using browser at: {:?}", account_name, browser_path);

        // Configure browser
        let mut builder = BrowserConfig::builder()
            .window_size(1920, 1080)
            .no_sandbox() // Add no-sandbox for compatibility
            .user_data_dir(&temp_dir) // Use unique user data directory
            .chrome_executable(&browser_path); // Use found browser

        // Set headless mode
        if !self.headless {
            builder = builder.with_head();
        }

        let config = builder.build().map_err(|e| {
            let err_msg = format!("Failed to build browser config: {}", e);
            log::error!("[{}] {}", account_name, err_msg);
            anyhow::anyhow!(err_msg)
        })?;

        info!("[{}] Browser config created, launching...", account_name);

        // Launch browser with timeout
        let timeout_config = TimeoutConfig::global();
        let launch_result =
            tokio::time::timeout(timeout_config.browser_launch, Browser::launch(config)).await;

        let (browser, mut handler) = match launch_result {
            Ok(Ok(browser_handler)) => browser_handler,
            Ok(Err(e)) => {
                // Clean up temp directory on failure
                let _ = std::fs::remove_dir_all(&temp_dir);
                let err_msg = format!(
                    "Failed to launch browser: {}. Make sure Chrome is installed and has proper permissions.",
                    e
                );
                log::error!("[{}] {}", account_name, err_msg);
                return Err(anyhow::anyhow!(err_msg));
            }
            Err(_) => {
                // Clean up temp directory on timeout
                let _ = std::fs::remove_dir_all(&temp_dir);
                let err_msg = "Browser launch timed out after 30 seconds".to_string();
                log::error!("[{}] {}", account_name, err_msg);
                return Err(anyhow::anyhow!(err_msg));
            }
        };

        info!("[{}] Browser launched successfully", account_name);

        // Spawn handler task and keep the handle for cleanup
        let handler_task = tokio::spawn(async move {
            while let Some(_event) = handler.next().await {
                // Handle events if needed
            }
        });

        Ok((browser, handler_task, temp_dir))
    }

    /// Navigate to page and extract WAF cookies
    /// Returns (browser, cookies_result) to allow cleanup even on error
    async fn navigate_and_extract_cookies(
        &self,
        browser: Browser,
        login_url: &str,
        account_name: &str,
    ) -> (Browser, Result<HashMap<String, String>>) {
        // Create new page
        let page = match browser.new_page("about:blank").await {
            Ok(p) => p,
            Err(e) => {
                let err_msg = format!("Failed to create new page: {}", e);
                log::error!("[{}] {}", account_name, err_msg);
                return (browser, Err(anyhow::anyhow!(err_msg)));
            }
        };

        info!("[{}] New page created", account_name);

        // Set user agent
        if let Err(e) = page.set_user_agent(USER_AGENT).await {
            let err_msg = format!("Failed to set user agent: {}", e);
            log::error!("[{}] {}", account_name, err_msg);
            return (browser, Err(anyhow::anyhow!(err_msg)));
        }

        info!("[{}] Navigating to: {}", account_name, login_url);

        // Navigate to login page
        if let Err(e) = page.goto(login_url).await {
            let err_msg = format!("Failed to navigate to login page: {}", e);
            log::error!("[{}] {}", account_name, err_msg);
            return (browser, Err(anyhow::anyhow!(err_msg)));
        }

        info!("[{}] Page loaded, waiting for WAF cookies...", account_name);

        // Wait for cookies to be set (configurable timeout)
        let timeout_config = TimeoutConfig::global();
        tokio::time::sleep(timeout_config.waf_wait).await;

        // Get all cookies
        let cookies = match page.get_cookies().await {
            Ok(c) => c,
            Err(e) => {
                let err_msg = format!("Failed to get cookies: {}", e);
                log::error!("[{}] {}", account_name, err_msg);
                return (browser, Err(anyhow::anyhow!(err_msg)));
            }
        };

        info!(
            "[{}] Retrieved {} cookies from browser",
            account_name,
            cookies.len()
        );

        // Extract WAF cookies
        let mut waf_cookies = HashMap::new();
        for cookie in cookies {
            let cookie_name = &cookie.name;
            let cookie_value = &cookie.value;

            info!(
                "[{}] Cookie found: {} = {}",
                account_name,
                cookie_name,
                mask_sensitive(cookie_value)
            );

            if REQUIRED_WAF_COOKIES.contains(&cookie_name.as_str()) {
                waf_cookies.insert(cookie_name.clone(), cookie_value.clone());
                info!("[{}] ✓ WAF cookie captured: {}", account_name, cookie_name);
            }
        }

        info!(
            "[{}] Captured {} WAF cookies out of {} required",
            account_name,
            waf_cookies.len(),
            REQUIRED_WAF_COOKIES.len()
        );

        (browser, Ok(waf_cookies))
    }
}

impl Default for WafBypassService {
    fn default() -> Self {
        Self::new(true) // Headless by default
    }
}

/// Check which browser is available on the system
pub fn check_available_browser() -> Option<String> {
    find_browser().map(|path| path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waf_service_creation() {
        let service = WafBypassService::new(true);
        assert!(service.headless);
    }

    #[test]
    fn test_browser_detection() {
        let browser = find_browser();
        println!("Found browser: {:?}", browser);
        // This test will pass even if no browser is found
        // It's just for checking during development
    }
}
