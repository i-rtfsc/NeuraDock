-- Add API Tokens table for token management feature
-- This stores cached token data fetched from /api/token endpoint

CREATE TABLE IF NOT EXISTS api_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL,  -- Foreign key to accounts.id

    -- Token basic information
    token_id INTEGER NOT NULL,  -- Token ID from API
    token_name TEXT NOT NULL,
    token_key TEXT NOT NULL,  -- API Key

    -- Status and quota
    status INTEGER NOT NULL DEFAULT 1,  -- 1=enabled, 2=disabled, 3=expired
    used_quota INTEGER NOT NULL DEFAULT 0,
    remain_quota INTEGER NOT NULL DEFAULT 0,
    unlimited_quota INTEGER NOT NULL DEFAULT 0,  -- 0=limited, 1=unlimited

    -- Time information
    expired_time INTEGER,  -- Unix timestamp, -1 means never expire

    -- Model limits (JSON storage)
    model_limits_allowed TEXT,  -- JSON array: ["gpt-4", "claude-3"]
    model_limits_denied TEXT,   -- JSON array: ["gpt-3.5"]

    -- Cache time
    fetched_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Constraints
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    UNIQUE (account_id, token_id)  -- One token_id per account
);

CREATE INDEX IF NOT EXISTS idx_api_tokens_account_id ON api_tokens(account_id);
CREATE INDEX IF NOT EXISTS idx_api_tokens_status ON api_tokens(status);
