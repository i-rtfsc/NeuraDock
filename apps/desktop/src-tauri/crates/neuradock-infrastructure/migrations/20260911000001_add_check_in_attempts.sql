-- Every check-in attempt, regardless of outcome.
--
-- Evidence (check_in_records) records business facts: "this quota day is
-- satisfied". Attempts record operations: "we tried, here is what happened".
-- Keeping them apart is what lets the app answer three different questions
-- without conflating them -- has today been satisfied (evidence), is a request
-- already in flight (attempt with no finished_at), and should we back off
-- (recent failed attempts).
CREATE TABLE check_in_attempts (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id   TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    provider_id  TEXT NOT NULL,
    quota_day    TEXT NOT NULL,
    trigger_kind TEXT NOT NULL CHECK (trigger_kind IN ('manual', 'batch', 'scheduler', 'observe')),
    started_at   TEXT NOT NULL,
    finished_at  TEXT,
    outcome      TEXT CHECK (outcome IS NULL OR outcome IN (
                     'success', 'already_confirmed', 'failed_retryable',
                     'failed_permanent', 'waf_blocked', 'skipped_guard', 'cancelled')),
    error_code   TEXT,
    http_status  INTEGER,
    duration_ms  INTEGER,
    reward_cents INTEGER,
    source       TEXT
);
CREATE INDEX idx_attempts_account_day ON check_in_attempts(account_id, quota_day);
CREATE INDEX idx_attempts_account_time ON check_in_attempts(account_id, started_at DESC);
