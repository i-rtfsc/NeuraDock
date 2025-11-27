# Check-In Operations

## Manual Check-In

### Single Account

1. Find the account card on the Accounts page
2. Click the **Check In** button
3. Wait for the operation to complete
4. View the result toast notification

### What Happens During Check-In

1. **Session Check**: Verifies if cached session is still valid
2. **WAF Bypass** (if needed): Launches browser to bypass Cloudflare
3. **API Request**: Sends check-in request to provider
4. **Balance Update**: Fetches and caches new balance
5. **History Record**: Saves check-in result to database

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
4. Save the account

### How Auto Check-In Works

- NeuraDock must be running for auto check-in to work
- Check-ins execute at the scheduled time each day
- If the app is closed at scheduled time, check-in is skipped
- Results are logged to check-in history

### Recommended Settings

| Setting | Recommendation |
|---------|----------------|
| Time | Early morning (6:00-9:00) to avoid peak hours |
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

1. Navigate to Dashboard
2. View recent check-ins in the activity feed
3. (Coming soon) Detailed history page

**Note**: Check-in history feature is not yet fully implemented.
