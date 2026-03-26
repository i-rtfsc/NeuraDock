-- Codex (OpenAI) account management
-- Stores registered/imported OpenAI accounts with their OAuth tokens

CREATE TABLE IF NOT EXISTS codex_accounts (
    id              TEXT PRIMARY KEY,
    email           TEXT NOT NULL UNIQUE,
    password        TEXT,                   -- Registration password (may be empty for imported)

    -- OAuth tokens
    access_token    TEXT,
    refresh_token   TEXT,
    id_token        TEXT,
    account_id      TEXT,                   -- OpenAI org/workspace ID (account_id in auth.json)

    -- Plan & quota (cached from /api/codex/usage)
    plan_type       TEXT,                   -- 'free' | 'plus' | 'pro' | 'team'
    has_credits     INTEGER,                -- 0 or 1
    credit_balance  TEXT,                   -- e.g. "$12.40"
    quota_checked_at TEXT,                  -- ISO8601 timestamp of last quota check

    -- Token expiry (decoded from access_token JWT)
    token_expires_at TEXT,                  -- ISO8601

    -- Last token refresh
    last_refresh_at  TEXT,                  -- ISO8601, mirrors auth.json last_refresh

    -- Tempmail inbox token (persisted so OTP polling survives restarts)
    tempmail_token  TEXT,

    -- Source and lifecycle
    source          TEXT NOT NULL DEFAULT 'register',  -- 'register' | 'import'
    status          TEXT NOT NULL DEFAULT 'active',    -- 'active' | 'expired' | 'banned'

    -- Timestamps
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_codex_accounts_email    ON codex_accounts(email);
CREATE INDEX IF NOT EXISTS idx_codex_accounts_status   ON codex_accounts(status);
CREATE INDEX IF NOT EXISTS idx_codex_accounts_plan     ON codex_accounts(plan_type);
