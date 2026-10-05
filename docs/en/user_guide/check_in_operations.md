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
4. Set the **minimum retry interval** as needed (optional; it only affects the retry pacing after a failure)
5. Save the account

### What the Scheduler Does (Depending on the Provider)

| Provider type | What the app does |
|---------------|-------------------|
| Exposes a check-in API (e.g. AnyRouter) | Sends a check-in request at the time you configured |
| Website-only check-in (e.g. AgentRouter) | **Sends no request**; instead it reads the balance once every 6 hours and records an Inferred check-in whenever it sees a quota increase |
| Marked as unavailable | Generates no task and produces no daily failure log |

### Minimum Retry Interval

- **Default 0**: uses the built-in 30-minute throttle
- **Configurable from 0-24 hours**: the shortest wait between attempts after a failure
- It **no longer decides "can I check in today"**. Whether today is already done is decided by the check-in record: as long as the day already has a record, no request is repeated
- Consecutive failures back off exponentially (30 minutes → 1 hour → 2 hours → 4 hours, capped at 6 hours) and never cross into the next check-in day

### How Auto Check-In Works

- NeuraDock must be running for auto check-in to work (including balance polling)
- Every attempt leaves a record: result, failure reason, and duration, viewable in History
- If the app is closed at the scheduled time, that attempt is skipped; while the day is still incomplete, the next run catches up
- Only one request runs at a time (a concurrent trigger is treated as "in flight" and skipped)
- A batch check-in can be cancelled in the interface; cancellation is cooperative, so requests already in flight run to completion

### Recommended Settings

| Setting | Recommendation |
|---------|----------------|
| Time | Early morning (6:00-9:00) to avoid peak hours |
| Minimum Retry Interval | Keep it at 0 (the default throttle); only raise it if you want to limit how often retries happen |
| Spread | Different times for different accounts to avoid rate limiting |

## Check-In Status

| Status | Meaning |
|--------|---------|
| Due | No record for today yet, and no request is running |
| Done | Today already has a record (from a provider confirmation, inference, or a manual entry) |
| In flight | A check-in request for this account is currently running |
| Backing off | The last attempt just failed and is waiting out the backoff; it will retry automatically when the wait expires |
| Skipped | Stopped by a guard (for example today is already done, or a request is already running) |

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
- **Trend Chart**: 30-day check-in and earnings trend on the account overview, 90-day on the account activity page
- **Calendar Deep-linking**: Click on dates to jump to detailed records
- **Missed Check-in Alerts**: Automatically identifies and marks missed check-ins
