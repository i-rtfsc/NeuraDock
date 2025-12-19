use anyhow::Result;
use chromiumoxide::browser::{Browser, BrowserConfig};
use futures::StreamExt;
use log::info;
use std::path::PathBuf;
use tokio::task::JoinHandle;

use crate::config::TimeoutConfig;

/// Find available Chromium-based browser on the system
pub(super) fn find_browser() -> Option<PathBuf> {
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

impl super::WafBypassService {
    /// Launch browser with proper configuration
    /// Returns (browser, handler_task, temp_dir)
    pub(super) async fn launch_browser_with_config(
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
}
