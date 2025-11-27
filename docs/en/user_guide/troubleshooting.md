# Troubleshooting

## Common Issues

### Check-In Failures

| Error | Cause | Solution |
|-------|-------|----------|
| "Session expired" | Cookies are no longer valid | Update cookies in account settings |
| "WAF challenge failed" | Cloudflare protection couldn't be bypassed | Ensure Chrome/Edge/Brave is installed |
| "Network error" | Connection issues | Check internet connection |
| "Invalid credentials" | Wrong cookies or API user | Verify credentials from browser |
| "Account not found" | Account was deleted or provider changed | Re-add the account |

### WAF Bypass Issues

**Problem**: Check-in fails with "WAF bypass failed"

**Solutions**:
1. Ensure a Chromium-based browser is installed:
   - Google Chrome
   - Microsoft Edge
   - Brave Browser

2. Check browser paths:
   - **macOS**: `/Applications/Google Chrome.app`
   - **Windows**: `C:\Program Files\Google\Chrome\Application\chrome.exe`
   - **Linux**: `/usr/bin/google-chrome` or `/usr/bin/chromium`

3. Close all browser instances and retry

4. Clear browser cache and cookies, then retry

**Problem**: Browser detected but WAF still fails

**Solutions**:
1. Update your browser to the latest version
2. Disable browser extensions that might interfere
3. Check if the service provider's website is accessible manually

### Balance Not Updating

**Problem**: Balance shows stale data

**Solutions**:
1. Click the refresh icon on the account card
2. Check Settings → Balance Cache Age setting
3. Verify the account has valid credentials
4. Check if the service provider's API is accessible

### Auto Check-In Not Working

**Problem**: Scheduled check-ins not executing

**Solutions**:
1. Verify auto check-in is enabled for the account
2. Check the scheduled time is correct
3. Ensure the application is running (not just in tray)
4. Check system time is correct
5. Look at check-in history for error messages

### Application Won't Start

**Problem**: Application crashes on launch

**Solutions**:

| Platform | Solution |
|----------|----------|
| macOS | Run `xattr -cr /Applications/NeuraDock.app` |
| Windows | Run as administrator |
| Linux | Check if WebKit2GTK is installed |

**If database is corrupted**:
1. Backup your accounts (export JSON if possible)
2. Delete the database file (see [Configuration](../configuration.md) for location)
3. Restart the application
4. Re-import your accounts

## Known Issues

### Critical (In Progress)

1. **Credentials stored unencrypted** - Security issue
   - Status: Under development
   - Workaround: None currently

2. **HTTP response body read twice** - Can cause check-in failures
   - Status: Fix pending
   - Workaround: Retry check-in

3. **Scheduler panic on invalid time** - Can crash scheduler
   - Status: Fix pending
   - Workaround: Ensure valid hour (0-23) and minute (0-59)

### Minor

1. **Balance may show stale data** after check-in if API fails
   - Workaround: Manual refresh

2. **Notification bell is placeholder** - Not functional
   - Status: Not implemented yet

3. **Some commands not implemented**:
   - `get_check_in_history`
   - `get_check_in_stats`
   - `stop_check_in`
   - `add_provider`

## Getting Debug Logs

To get detailed logs for troubleshooting:

**macOS/Linux**:
```bash
# Run from terminal with debug logging
RUST_LOG=debug /Applications/NeuraDock.app/Contents/MacOS/NeuraDock 2>&1 | tee neuradock.log
```

**Windows**:
```powershell
# Run from PowerShell
$env:RUST_LOG="debug"
& "C:\Program Files\NeuraDock\NeuraDock.exe" 2>&1 | Tee-Object -FilePath neuradock.log
```

## Reporting Issues

When reporting issues, please include:

1. **NeuraDock version** (Settings → About)
2. **Operating system** and version
3. **Steps to reproduce** the issue
4. **Error messages** or screenshots
5. **Debug logs** if available

Report issues at: https://github.com/neuradock/neuradock/issues
