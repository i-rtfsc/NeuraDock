-- Migration: Add performance indexes
-- Created: 2025-11-30
-- Purpose: Optimize query performance for frequently accessed data

-- ========================================
-- Account table indexes
-- ========================================

-- Index for filtering enabled accounts (used in scheduler and queries)
CREATE INDEX IF NOT EXISTS idx_accounts_enabled 
ON accounts(enabled) 
WHERE enabled = 1;

-- Index for provider-based queries
CREATE INDEX IF NOT EXISTS idx_accounts_provider 
ON accounts(provider_id);

-- Compound index for auto check-in scheduling
-- Used by scheduler to find accounts that need auto check-in
CREATE INDEX IF NOT EXISTS idx_accounts_auto_checkin 
ON accounts(auto_checkin_enabled, auto_checkin_hour, auto_checkin_minute)
WHERE enabled = 1 AND auto_checkin_enabled = 1;

-- Index for last check-in time queries (for streak calculations)
CREATE INDEX IF NOT EXISTS idx_accounts_last_checkin
ON accounts(last_check_in DESC);

-- Index for balance staleness checks
CREATE INDEX IF NOT EXISTS idx_accounts_balance_check
ON accounts(last_balance_check_at DESC)
WHERE last_balance_check_at IS NOT NULL;

-- ========================================
-- Check-in jobs table indexes
-- ========================================

-- Compound index for account-based job queries with status filter
CREATE INDEX IF NOT EXISTS idx_check_in_jobs_account_status 
ON check_in_jobs(account_id, status, scheduled_at DESC);

-- Index for status-based queries (finding pending/running jobs)
CREATE INDEX IF NOT EXISTS idx_check_in_jobs_status
ON check_in_jobs(status, scheduled_at DESC);

-- Index for provider-based queries
CREATE INDEX IF NOT EXISTS idx_check_in_jobs_provider
ON check_in_jobs(provider_id);

-- Index for time-range queries
CREATE INDEX IF NOT EXISTS idx_check_in_jobs_scheduled
ON check_in_jobs(scheduled_at DESC);

-- ========================================
-- Balance history table indexes
-- ========================================

-- Compound index for account-based balance history queries
CREATE INDEX IF NOT EXISTS idx_balance_history_account_time 
ON balance_history(account_id, recorded_at DESC);

-- Index for time-range queries
CREATE INDEX IF NOT EXISTS idx_balance_history_recorded
ON balance_history(recorded_at DESC);
