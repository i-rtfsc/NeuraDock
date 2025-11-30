-- ============================================================
-- NeuraDock Initial Database Schema
-- ============================================================

-- Create accounts table
CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    cookies TEXT NOT NULL,
    api_user TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    last_check_in TIMESTAMP,
    -- Auto check-in settings
    auto_checkin_enabled BOOLEAN NOT NULL DEFAULT 0,
    auto_checkin_hour INTEGER NOT NULL DEFAULT 9,
    auto_checkin_minute INTEGER NOT NULL DEFAULT 0,
    -- Session caching
    last_login_at TIMESTAMP,
    session_token TEXT,
    session_expires_at TIMESTAMP,
    -- Balance caching (from /api/user/self: quota=current_balance, used_quota=total_consumed, total_income=current_balance+total_consumed)
    last_balance_check_at TIMESTAMP,
    current_balance REAL,
    total_consumed REAL,
    total_income REAL,
    -- Timestamps
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create providers table
CREATE TABLE IF NOT EXISTS providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    domain TEXT NOT NULL,
    login_path TEXT NOT NULL,
    sign_in_path TEXT,
    user_info_path TEXT NOT NULL,
    api_user_key TEXT NOT NULL,
    bypass_method TEXT,
    is_builtin BOOLEAN NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL
);

-- Create check_in_jobs table
CREATE TABLE IF NOT EXISTS check_in_jobs (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    status TEXT NOT NULL,
    scheduled_at TIMESTAMP NOT NULL,
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    result_json TEXT,
    error TEXT,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    FOREIGN KEY (provider_id) REFERENCES providers(id)
);

-- Create balance_history table
CREATE TABLE IF NOT EXISTS balance_history (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    current_balance REAL NOT NULL,
    total_consumed REAL NOT NULL,
    total_income REAL NOT NULL,
    recorded_at TIMESTAMP NOT NULL,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

-- Create notification_channels table
CREATE TABLE IF NOT EXISTS notification_channels (
    id TEXT PRIMARY KEY,
    channel_type TEXT NOT NULL,
    config TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL
);

-- Create notification_history table
CREATE TABLE IF NOT EXISTS notification_history (
    id TEXT PRIMARY KEY,
    channel_id TEXT NOT NULL,
    message TEXT NOT NULL,
    status TEXT NOT NULL,
    error TEXT,
    sent_at TIMESTAMP NOT NULL,
    FOREIGN KEY (channel_id) REFERENCES notification_channels(id) ON DELETE CASCADE
);

-- ============================================================
-- Indexes
-- ============================================================

-- Account indexes
CREATE INDEX IF NOT EXISTS idx_accounts_enabled ON accounts(enabled) WHERE enabled = 1;
CREATE INDEX IF NOT EXISTS idx_accounts_provider ON accounts(provider_id);
CREATE INDEX IF NOT EXISTS idx_accounts_auto_checkin ON accounts(auto_checkin_enabled, auto_checkin_hour, auto_checkin_minute) WHERE auto_checkin_enabled = 1;
CREATE INDEX IF NOT EXISTS idx_accounts_session_expiry ON accounts(session_expires_at) WHERE session_expires_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_accounts_balance_check ON accounts(last_balance_check_at) WHERE last_balance_check_at IS NOT NULL;

-- Job indexes
CREATE INDEX IF NOT EXISTS idx_jobs_account ON check_in_jobs(account_id);
CREATE INDEX IF NOT EXISTS idx_jobs_status ON check_in_jobs(status);
CREATE INDEX IF NOT EXISTS idx_jobs_scheduled ON check_in_jobs(scheduled_at);

-- Balance history index
CREATE INDEX IF NOT EXISTS idx_balance_account_time ON balance_history(account_id, recorded_at DESC);

-- Notification index
CREATE INDEX IF NOT EXISTS idx_notification_channel ON notification_history(channel_id);

-- ============================================================
-- Built-in Providers
-- ============================================================

INSERT OR IGNORE INTO providers (id, name, domain, login_path, sign_in_path, user_info_path, api_user_key, bypass_method, is_builtin, created_at)
VALUES
('anyrouter', 'AnyRouter', 'https://anyrouter.top', '/login', '/api/user/sign_in', '/api/user/self', 'new-api-user', 'waf_cookies', 1, CURRENT_TIMESTAMP),
('agentrouter', 'AgentRouter', 'https://agentrouter.org', '/login', NULL, '/api/user/self', 'new-api-user', NULL, 1, CURRENT_TIMESTAMP);
