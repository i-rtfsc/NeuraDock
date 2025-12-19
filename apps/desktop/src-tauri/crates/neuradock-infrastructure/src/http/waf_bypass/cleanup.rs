use chromiumoxide::browser::Browser;
use log::{info, warn};
use std::path::PathBuf;
use std::time::Duration;
use tokio::task::JoinHandle;

use crate::config::TimeoutConfig;

/// Helper to clean up browser resources with timeout
pub(super) async fn cleanup_browser(
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

        let _temp_dir_str = temp_dir.to_string_lossy();

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
