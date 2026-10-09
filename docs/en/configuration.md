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
  "session": "your_session_token"
}
```

> **Note**: Only the `session` cookie is required. Other cookies will be automatically obtained at runtime.

**How to obtain cookies:**
1. Log in to your service provider's website
2. Open browser DevTools (F12)
3. Go to Application → Cookies
4. Copy the `session` cookie value

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

## Token Configuration

### AI Tool Configuration

NeuraDock can configure tokens for AI development tools, supporting:

| Tool | Config File | Description |
|------|-------------|-------------|
| **Claude Code** | `~/.claude/settings.json` | Anthropic Claude CLI tool |
| **Codex** | `~/.codex/config.toml` or `~/.codex/profiles.d/<profile>.config.toml` | Default and named profiles |

### Configuring Tokens

1. Navigate to **Token Manager** page
2. Select an account (AnyRouter or AgentRouter)
3. Click **Configure** button next to the token
4. Select AI tool and node
5. Click **Apply Configuration**

### Codex Accounts and Model Catalogs

Open **Agent Configuration → Codex → Add profile**, choose a relay, account, key and node, use an isolated configuration, select multiple models, and choose one default model.

Profile names combine the relay abbreviation and the account's email domain prefix: `any-qq` for an AnyRouter QQ email account, or `agent-gmail` for an AgentRouter Gmail account. Duplicate names receive a numeric suffix. Enter a domain prefix manually if the account name does not contain an email. Existing names are preserved when editing.

Each profile writes `~/.codex/profiles.d/any-qq.config.toml` and `any-qq.models.json`, with a symbolic link at `~/.codex/any-qq.config.toml`. Launch it with `codex -p any-qq`. Relay profiles use the shared provider ID `3rd` and the same Codex home directory.

Claude Code uses the same account naming: `~/.claude/profiles.d/any-qq.json`, launched with `claude --settings ~/.claude/profiles.d/any-qq.json`. Existing `settings-*.json` profiles retain their original paths.

Choose models from the server-provided list. Use “Refresh models” if it is empty or fails to load; model names cannot be added manually. Independent keys fetch the list from their configured model endpoint.

The list is filtered by the agent's protocol: Codex does not offer Claude models, and the backend rejects them as well. Use “More actions → Delete profile” to remove one named profile, its catalog, symbolic link and binding record. The default profile offers “Clear access configuration” while preserving other settings.

AgentRouter offers `https://agentrouter.org` and `https://ps.air-outer.com` as request nodes. AnyRouter retains its Shanghai node; saved records for its retired rainapp node are removed when the updated application starts.

DeepSeek V4 Flash and Pro use the supplied model templates. Other models receive the fields needed for a valid catalog. Each model entry defines its context and reasoning capabilities. Restart the corresponding Codex session after changing its catalog.

### Custom Nodes

If you have custom API endpoints, you can add custom nodes:

1. Click **Manage Nodes** button
2. Select service provider
3. Enter node name and URL
4. Click **Add**

**Example node configuration:**
```
Name: My Private Node
URL: https://api.example.com
```

### Clear Configuration

To clear configurations managed by NeuraDock:

1. Click **Clear Config** button
2. Select tool to clear (Claude Code or Codex)
3. Confirm operation



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
