# Check-In Operations

## Manual Check-In

### Single Account

1. Find the account card on the Accounts page
2. Click the **Check In** button
3. Wait for the operation to complete
4. View the result toast notification

### What Happens During Check-In

1. **Session Check**: Verifies if cached session is still valid
2. **Proxy Configuration** (if enabled): Sends requests through configured proxy server
3. **WAF Bypass** (if needed): Launches browser to bypass Cloudflare
4. **API Request**: Sends check-in request to provider
5. **Balance Update**: Fetches and caches new balance
6. **History Record**: Saves check-in result to database

### v0.5.0 Feature: Proxy Configuration

You can configure application proxy in the Preferences page:
- **Proxy Type**: Supports HTTP, HTTPS, and SOCKS5
- **Proxy Address**: Format is `host:port`, e.g., `127.0.0.1:7890`
- **Scope**: All check-in, balance query, and token fetch requests will go through the proxy

## Batch Check-In

Check in multiple accounts at once:

1. On the Accounts page, select accounts using checkboxes
2. Click **Batch Check-In** button
3. Monitor progress in the dialog
4. Review results when complete

**Tips**:
- Disabled accounts are excluded from batch operations
- Failed check-ins don't stop the batch - all accounts are attempted
- Results show success/failure for each account

## Auto Check-In

### Enabling Auto Check-In

1. Edit the account
2. Toggle **Auto Check-In** to enabled
3. Set the check-in time (hour and minute)
4. Configure check-in interval (optional, default is no limit)
5. Save the account

### v0.5.0 Feature: Configurable Check-in Interval

You can now set custom check-in intervals for each account to avoid rate limiting:

- **Default interval**: 0 hours (no limit, executes only at scheduled time)
- **Custom interval**: Can be set between 0-24 hours (0 means no time restriction)
- **Use case**: Adapt to different provider requirements, e.g., set 24 hours to ensure check-in once per day

### How Auto Check-In Works

- NeuraDock must be running for auto check-in to work
- Check-ins execute according to configured interval (not fixed daily)
- If the app is closed at scheduled time, check-in is skipped
- Results are logged to check-in history, viewable in Check-in Streaks page

### Recommended Settings

| Setting | Recommendation |
|---------|----------------|
| Time | Early morning (6:00-9:00) to avoid peak hours |
| Interval | Set to 24 hours to ensure check-in only once per day and avoid duplicate check-ins |
| Spread | Different times for different accounts to avoid rate limiting |

## Check-In Status

| Status | Meaning |
|--------|---------|
| ✅ Success | Check-in completed successfully |
| ❌ Failed | Check-in failed (see error message) |
| 🔄 In Progress | Check-in is currently running |
| ⏳ Scheduled | Auto check-in is scheduled |

## Common Check-In Errors

| Error | Cause | Solution |
|-------|-------|----------|
| Session expired | Cookies are invalid | Update cookies |
| WAF bypass failed | Browser automation failed | Check browser installation |
| Network error | Connection issues | Check internet |
| Already checked in | Duplicate check-in today | Wait until tomorrow |
| Rate limited | Too many requests | Wait and retry later |

## Check-In History

View past check-in records:

1. Navigate to **Check-in Streaks** page
2. Select the account to view
3. View streak statistics, calendar view, and trend analysis
4. Click on calendar dates to view detailed records

### v0.5.0 Check-in Analytics

The Check-in Streaks page now provides advanced analytics:

- **Monthly Overview**: View current month's check-in status and earnings
- **Trend Chart**: 30-day check-in and earnings trend visualization
- **Calendar Deep-linking**: Click on dates to jump to detailed records
- **Missed Check-in Alerts**: Automatically identifies and marks missed check-ins
