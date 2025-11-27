# Configuration Guide

## Application Settings

NeuraDock settings can be accessed via **Settings** page in the sidebar.

### General Settings

| Setting | Description | Default |
|---------|-------------|---------|
| Language | Interface language (English/中文) | System language |
| Theme | Light/Dark/System | System |
| Balance Cache Age | Hours before balance is considered stale | 1 hour |

### Auto Check-In Settings

Configure automatic check-in behavior per account:

| Setting | Description |
|---------|-------------|
| Enable Auto Check-In | Toggle automatic check-in for this account |
| Check-In Time | Hour and minute to execute check-in (24h format) |

## Account Configuration

### Adding an Account

1. Navigate to **Accounts** page
2. Click **Add Account** button
3. Fill in the required fields:
   - **Name**: Display name for the account
   - **Provider**: Service provider (AnyRouter, AgentRouter)
   - **Cookies**: Authentication cookies (JSON format)
   - **API User**: API user identifier

### Cookie Format

Cookies should be provided in JSON format:

```json
{
  "session": "your_session_token",
  "cf_clearance": "cloudflare_clearance_cookie"
}
```

**How to obtain cookies:**
1. Log in to your service provider's website
2. Open browser DevTools (F12)
3. Go to Application → Cookies
4. Copy the required cookie values

### Batch Import/Export

**Import from JSON:**
```json
[
  {
    "name": "account@example.com",
    "provider": "anyrouter",
    "cookies": {
      "session": "token_value"
    },
    "api_user": "user_id"
  }
]
```

**Export accounts:**
- Navigate to Accounts page
- Select accounts to export
- Click Export button
- Choose whether to include credentials

## Database Location

NeuraDock stores data in SQLite database:

| Platform | Location |
|----------|----------|
| macOS | `~/Library/Application Support/com.neuradock.app/neuradock.db` |
| Windows | `%APPDATA%\com.neuradock.app\neuradock.db` |
| Linux | `~/.local/share/com.neuradock.app/neuradock.db` |

**Development database**: `neuradock_dev.db` (same directory)

## Provider Configuration

Built-in providers are configured in the application:

| Provider | Base URL | WAF Bypass |
|----------|----------|------------|
| AnyRouter | `https://api.anyrouter.com` | Required |
| AgentRouter | `https://api.agentrouter.com` | Not Required |

## Environment Variables

Currently, NeuraDock does not use environment variables. All configuration is stored in the application settings and database.

## Advanced Configuration

### Custom Browser Path (WAF Bypass)

NeuraDock automatically detects Chromium browsers. Supported browsers:
- Google Chrome
- Microsoft Edge
- Brave Browser
- Chromium

Detection paths:
- **macOS**: `/Applications/Google Chrome.app`, `/Applications/Brave Browser.app`
- **Windows**: Registry and Program Files
- **Linux**: `/usr/bin/google-chrome`, `/usr/bin/chromium`
