# Balance Tracking

## Viewing Balance

Balance information is displayed on each account card:

- **Quota**: Total allocated quota (GB)
- **Used**: Amount used this period
- **Remaining**: Available quota

## Balance Caching

To minimize API requests, NeuraDock caches balance data:

| Setting | Default | Description |
|---------|---------|-------------|
| Cache Age | 1 hour | Time before balance is considered stale |

### How Caching Works

1. When you view an account, balance is fetched if stale
2. After check-in, balance is automatically refreshed
3. Manual refresh fetches fresh data regardless of cache

### Configuring Cache Age

1. Go to **Settings** page
2. Find **Balance Cache Age** setting
3. Adjust the hours (1-24)
4. Changes apply immediately

## Manual Refresh

To force-refresh an account's balance:

1. Find the account card
2. Click the **🔄** (refresh) icon
3. Wait for the balance to update

## Balance History

NeuraDock tracks balance over time:

- One record per day maximum
- Stored in the database
- (Coming soon) Balance history chart

## Understanding Balance Values

| Value | Meaning |
|-------|---------|
| Quota | Total GB allocated for the billing period |
| Used | GB consumed so far |
| Remaining | Quota - Used |
| Last Check | When balance was last fetched |
| Fresh | Whether data is within cache age |

## Balance Indicators

| Indicator | Meaning |
|-----------|---------|
| Green | > 50% remaining |
| Yellow | 20-50% remaining |
| Red | < 20% remaining |
| Gray | Balance unknown |

## Troubleshooting Balance Issues

**Balance shows "--" or "Unknown"**:
- Account has never been checked
- API request failed
- Click refresh to fetch balance

**Balance seems wrong**:
- Cache may be stale - click refresh
- Provider API may be delayed
- Check provider website directly

**Balance not updating after check-in**:
- Check if check-in was successful
- Some providers delay balance updates
- Manual refresh after a few minutes
