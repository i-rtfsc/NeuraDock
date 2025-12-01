-- Remove legacy session and balance fields from accounts table
-- These fields have been migrated to separate sessions and balances tables

-- Note: ALTER TABLE DROP COLUMN is supported in SQLite 3.35.0+
-- For older versions, this migration will need to recreate the table

-- IMPORTANT: Must drop indexes BEFORE dropping columns they reference
DROP INDEX IF EXISTS idx_accounts_session_expiry;
DROP INDEX IF EXISTS idx_accounts_balance_check;

-- Remove session-related fields
ALTER TABLE accounts DROP COLUMN last_login_at;
ALTER TABLE accounts DROP COLUMN session_token;
ALTER TABLE accounts DROP COLUMN session_expires_at;

-- Remove balance-related fields
ALTER TABLE accounts DROP COLUMN last_balance_check_at;
ALTER TABLE accounts DROP COLUMN current_balance;
ALTER TABLE accounts DROP COLUMN total_consumed;
ALTER TABLE accounts DROP COLUMN total_income;
