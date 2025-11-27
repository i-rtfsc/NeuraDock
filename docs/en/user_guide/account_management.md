# Account Management

## Adding an Account

1. Navigate to **Accounts** page from the sidebar
2. Click the **Add Account** button
3. Fill in the account form:

| Field | Required | Description |
|-------|----------|-------------|
| Name | Yes | Display name (usually email address) |
| Provider | Yes | Service provider (AnyRouter, AgentRouter) |
| Cookies | Yes | Authentication cookies in JSON format |
| API User | No | API user identifier (provider-specific) |
| Auto Check-In | No | Enable automatic daily check-in |
| Check-In Time | No | Time for auto check-in (24h format) |

4. Click **Save** to create the account

## Getting Cookies

1. Log in to your service provider's website in a browser
2. Open Developer Tools (F12 or Cmd+Option+I)
3. Go to **Application** tab → **Cookies**
4. Copy the required cookie values
5. Format as JSON:

```json
{
  "session": "your_session_token_here",
  "cf_clearance": "cloudflare_cookie_if_needed"
}
```

## Editing an Account

1. Find the account card on the Accounts page
2. Click the **⋮** (more) menu on the card
3. Select **Edit**
4. Modify the desired fields
5. Click **Save**

## Deleting an Account

1. Find the account card
2. Click the **⋮** menu
3. Select **Delete**
4. Confirm the deletion

**Warning**: Deleting an account also deletes its check-in history and balance records.

## Enabling/Disabling Accounts

- Toggle the switch on the account card to enable/disable
- Disabled accounts won't appear in batch operations
- Auto check-in is paused for disabled accounts

## Batch Import

Import multiple accounts at once using JSON:

1. Click **Import** button on Accounts page
2. Paste JSON array:

```json
[
  {
    "name": "account1@example.com",
    "provider": "anyrouter",
    "cookies": {
      "session": "token1"
    },
    "api_user": "user1"
  },
  {
    "name": "account2@example.com",
    "provider": "agentrouter",
    "cookies": {
      "session": "token2"
    }
  }
]
```

3. Click **Import**
4. Review results

## Batch Update

Update credentials for multiple existing accounts:

1. Click **Batch Update** on Accounts page
2. Paste JSON with updated credentials
3. Optionally enable "Create if not exists"
4. Click **Update**

## Exporting Accounts

1. Select accounts to export (or export all)
2. Click **Export** button
3. Choose whether to include credentials
4. Save the JSON file

**Security Note**: Exported files with credentials contain sensitive data. Store securely.
