-- Create sessions table
CREATE TABLE IF NOT EXISTS sessions (
    account_id TEXT PRIMARY KEY,
    token TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    last_login_at TEXT NOT NULL,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);

-- Create balances table
CREATE TABLE IF NOT EXISTS balances (
    account_id TEXT PRIMARY KEY,
    current REAL NOT NULL DEFAULT 0.0,
    total_consumed REAL NOT NULL DEFAULT 0.0,
    total_income REAL NOT NULL DEFAULT 0.0,
    last_checked_at TEXT NOT NULL,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_balances_last_checked ON balances(last_checked_at);
CREATE INDEX IF NOT EXISTS idx_balances_current ON balances(current);

-- Migrate existing session data from accounts table
INSERT INTO sessions (account_id, token, expires_at, last_login_at)
SELECT 
    id,
    COALESCE(session_token, ''),
    COALESCE(session_expires_at, datetime('now')),
    COALESCE(last_login_at, datetime('now'))
FROM accounts
WHERE session_token IS NOT NULL AND session_token != '';

-- Migrate existing balance data from accounts table
INSERT INTO balances (account_id, current, total_consumed, total_income, last_checked_at)
SELECT 
    id,
    COALESCE(current_balance, 0.0),
    COALESCE(total_consumed, 0.0),
    COALESCE(total_income, 0.0),
    COALESCE(last_balance_check_at, datetime('now'))
FROM accounts;

-- Optional: Remove migrated columns from accounts table
-- Uncomment these lines if you want to remove the legacy columns
-- ALTER TABLE accounts DROP COLUMN session_token;
-- ALTER TABLE accounts DROP COLUMN session_expires_at;
-- ALTER TABLE accounts DROP COLUMN last_login_at;
-- ALTER TABLE accounts DROP COLUMN current_balance;
-- ALTER TABLE accounts DROP COLUMN total_consumed;
-- ALTER TABLE accounts DROP COLUMN total_income;
-- ALTER TABLE accounts DROP COLUMN last_balance_check_at;

-- Note: SQLite doesn't support DROP COLUMN directly in older versions
-- You would need to:
-- 1. Create a new accounts table without these columns
-- 2. Copy data from old table
-- 3. Drop old table
-- 4. Rename new table
-- This is left as a manual step if needed
