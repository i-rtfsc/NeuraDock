use chromiumoxide::browser::Browser;
use log::{info, warn};
use std::path::{Path, PathBuf};
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

    // Remember the OS pid before we consume the browser, so a forced cleanup
    // can target exactly this Chrome rather than every Chrome on the machine.
    let browser_pid = browser.get_mut_child().map(|child| child.inner.id());

    // Order matters. `Browser::close()` does not talk to Chrome directly: it
    // sends CloseBrowser over an mpsc channel whose receiver lives inside the
    // Handler, then awaits a oneshot reply. Aborting the handler first (as this
    // did) drops that receiver, so close() could never succeed and Chrome was
    // only ever killed by kill_on_drop's SIGKILL. That left the profile lock
    // held, which made remove_dir_all fail, which is what dragged every run
    // into force_kill_chrome_processes below.
    //
    // close -> wait -> abort keeps the Handler alive long enough to drive the
    // CDP shutdown to completion.
    match tokio::time::timeout(config.browser_close, browser.close()).await {
        Ok(Ok(_)) => {
            info!("[{}] Browser closed successfully", account_name);
            // Reap the child so the profile lock is actually released before
            // we try to remove the directory.
            if let Err(e) = tokio::time::timeout(config.browser_close, browser.wait()).await {
                warn!(
                    "[{}] Browser did not exit within timeout after close: {}",
                    account_name, e
                );
            }
        }
        Ok(Err(e)) => {
            warn!(
                "[{}] Failed to close browser: {}, will force cleanup",
                account_name, e
            );
        }
        Err(_) => {
            warn!(
                "[{}] Browser close timed out, continuing with cleanup",
                account_name
            );
        }
    }

    // Only now is it safe to tear down the connection driver.
    handler_task.abort();

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
        force_kill_chrome_processes(&temp_dir, browser_pid, account_name).await;

        // Wait a bit more and retry cleanup
        tokio::time::sleep(Duration::from_secs(2)).await;

        if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
            warn!(
                "[{}] Failed to clean up temp directory even after force kill: {}",
                account_name, e
            );
        } else {
            info!(
                "[{}] Cleaned up temp profile directory after force kill",
                account_name
            );
        }
    } else {
        info!("[{}] Cleaned up temp profile directory", account_name);
    }
}

/// Force kill the Chrome process that might still be holding the temp profile.
///
/// `browser_pid` is the pid of the browser this call launched. It is used to
/// scope the kill; when it is unknown we do nothing rather than guess.
async fn force_kill_chrome_processes(
    temp_dir: &Path,
    browser_pid: Option<u32>,
    account_name: &str,
) {
    #[cfg(unix)]
    {
        use std::process::Command;

        // On unix the profile path is the scoping mechanism: lsof tells us
        // exactly which processes hold this directory, so the pid is unused.
        let _ = browser_pid;

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
                    let _ = Command::new("kill").arg("-9").arg(pid.trim()).output();
                }
            }
        }
    }

    #[cfg(windows)]
    {
        use std::process::Command;

        let _ = temp_dir;

        // Kill ONLY the browser we launched, plus its renderer/GPU children
        // (/T). This used to be `taskkill /F /IM chrome.exe`, which force-killed
        // every Chrome on the machine -- including the user's foreground
        // browser, all tabs and unsaved work -- during a background check-in.
        match browser_pid {
            Some(pid) => {
                warn!(
                    "[{}] Force killing Chrome process tree with PID: {}",
                    account_name, pid
                );
                let _ = Command::new("taskkill")
                    .args(["/F", "/T", "/PID", &pid.to_string()])
                    .output();
            }
            None => {
                // Never fall back to a machine-wide kill. The stray profile
                // directory is collected by the startup sweep instead.
                warn!(
                    "[{}] Chrome pid unknown; leaving temp profile for startup cleanup",
                    account_name
                );
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = (temp_dir, browser_pid, account_name);
    }
}
