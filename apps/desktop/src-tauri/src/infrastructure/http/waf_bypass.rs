use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use anyhow::{Result, Context};
use chromiumoxide::browser::{Browser, BrowserConfig};
use futures::StreamExt;
use log::{info, warn};

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36";
const REQUIRED_WAF_COOKIES: &[&str] = &["acw_tc", "cdn_sec_tc", "acw_sc__v2"];

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
        let windows_paths = vec![
            // Chrome (Program Files)
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
            // Chrome (Local AppData)
            format!(r"{}\Google\Chrome\Application\chrome.exe", 
                    std::env::var("LOCALAPPDATA").unwrap_or_default()),
            // Chromium
            r"C:\Program Files\Chromium\Application\chrome.exe",
            r"C:\Program Files (x86)\Chromium\Application\chrome.exe",
            // Brave
            r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe",
            r"C:\Program Files (x86)\BraveSoftware\Brave-Browser\Application\brave.exe",
            format!(r"{}\BraveSoftware\Brave-Browser\Application\brave.exe",
                    std::env::var("LOCALAPPDATA").unwrap_or_default()),
            // Microsoft Edge
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
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
            "/ve"
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
    pub async fn get_waf_cookies(&self, login_url: &str, account_name: &str) -> Result<HashMap<String, String>> {
        const MAX_RETRIES: u32 = 2;
        let mut last_error = None;
        
        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                info!("[{}] Retrying WAF cookie fetch (attempt {}/{})", account_name, attempt + 1, MAX_RETRIES);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            
            match self.get_waf_cookies_once(login_url, account_name).await {
                Ok(cookies) => return Ok(cookies),
                Err(e) => {
                    warn!("[{}] WAF cookie fetch attempt {} failed: {}", account_name, attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Failed to get WAF cookies after {} attempts", MAX_RETRIES)))
    }
    
    /// Internal method to get WAF cookies once
    async fn get_waf_cookies_once(&self, login_url: &str, account_name: &str) -> Result<HashMap<String, String>> {
        info!("[{}] Starting browser to get WAF cookies (chromiumoxide)...", account_name);

        // Use unique temporary directory for each session to avoid lock conflicts
        let temp_dir = std::env::temp_dir().join(format!("chromiumoxide-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create temp directory: {}", e))?;
        
        info!("[{}] Using temp profile directory: {:?}", account_name, temp_dir);

        // Find available browser
        let browser_path = find_browser()
            .ok_or_else(|| {
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
        
        let config = builder.build()
            .map_err(|e| {
                let err_msg = format!("Failed to build browser config: {}", e);
                log::error!("[{}] {}", account_name, err_msg);
                anyhow::anyhow!(err_msg)
            })?;

        info!("[{}] Browser config created, launching...", account_name);

        // Launch browser with timeout
        let launch_result = tokio::time::timeout(
            Duration::from_secs(30),
            Browser::launch(config)
        ).await;
        
        let (mut browser, mut handler) = match launch_result {
            Ok(Ok(browser_handler)) => browser_handler,
            Ok(Err(e)) => {
                // Clean up temp directory on failure
                let _ = std::fs::remove_dir_all(&temp_dir);
                let err_msg = format!("Failed to launch browser: {}. Make sure Chrome is installed and has proper permissions.", e);
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

        // Spawn handler task
        let _handler_task = tokio::spawn(async move {
            while let Some(_event) = handler.next().await {
                // Handle events if needed
            }
        });

        // Create new page
        let page = browser.new_page("about:blank")
            .await
            .map_err(|e| {
                let err_msg = format!("Failed to create new page: {}", e);
                log::error!("[{}] {}", account_name, err_msg);
                anyhow::anyhow!(err_msg)
            })?;

        info!("[{}] New page created", account_name);

        // Set user agent
        page.set_user_agent(USER_AGENT)
            .await
            .map_err(|e| {
                let err_msg = format!("Failed to set user agent: {}", e);
                log::error!("[{}] {}", account_name, err_msg);
                anyhow::anyhow!(err_msg)
            })?;

        info!("[{}] Navigating to: {}", account_name, login_url);

        // Navigate to login page
        page.goto(login_url)
            .await
            .map_err(|e| {
                let err_msg = format!("Failed to navigate to login page: {}", e);
                log::error!("[{}] {}", account_name, err_msg);
                anyhow::anyhow!(err_msg)
            })?;

        info!("[{}] Page loaded, waiting for WAF cookies...", account_name);

        // Wait for cookies to be set (increased timeout)
        tokio::time::sleep(Duration::from_secs(8)).await;

        // Get all cookies
        let cookies = page.get_cookies()
            .await
            .map_err(|e| {
                let err_msg = format!("Failed to get cookies: {}", e);
                log::error!("[{}] {}", account_name, err_msg);
                anyhow::anyhow!(err_msg)
            })?;

        info!("[{}] Retrieved {} cookies from browser", account_name, cookies.len());

        // Extract WAF cookies
        let mut waf_cookies = HashMap::new();
        for cookie in cookies {
            let cookie_name = &cookie.name;
            let cookie_value = &cookie.value;

            info!("[{}] Cookie found: {} = {}...", account_name, cookie_name,
                  if cookie_value.len() > 10 { &cookie_value[..10] } else { cookie_value });

            if REQUIRED_WAF_COOKIES.contains(&cookie_name.as_str()) {
                waf_cookies.insert(cookie_name.clone(), cookie_value.clone());
                info!("[{}] ✓ WAF cookie captured: {}", account_name, cookie_name);
            }
        }

        info!("[{}] Captured {} WAF cookies out of {} required", account_name, waf_cookies.len(), REQUIRED_WAF_COOKIES.len());

        // Close browser
        if let Err(e) = browser.close().await {
            warn!("[{}] Failed to close browser cleanly: {}", account_name, e);
        }
        
        // Clean up temp directory
        tokio::time::sleep(Duration::from_secs(1)).await; // Give Chrome time to exit
        if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
            warn!("[{}] Failed to clean up temp directory: {}", account_name, e);
        } else {
            info!("[{}] Cleaned up temp profile directory", account_name);
        }

        // Check if we got any cookies
        if waf_cookies.is_empty() {
            let err_msg = format!("No WAF cookies obtained. Expected cookies: {:?}. This might indicate that the page didn't load properly or WAF protection has changed.", REQUIRED_WAF_COOKIES);
            warn!("[{}] {}", account_name, err_msg);
            // Return error instead of empty map
            anyhow::bail!(err_msg);
        } else {
            info!("[{}] ✓ Successfully got {} WAF cookies", account_name, waf_cookies.len());
        }

        Ok(waf_cookies)
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
