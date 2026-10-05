-- Providers declare how their quota day is bucketed and whether the local app
-- may POST a check-in at all.
--
-- A provider with no sign-in endpoint is observation-only: the app never posts,
-- it detects the web check-in from the quota delta on the next balance
-- observation. `check_in_bugged` stays a separate, temporary switch.
ALTER TABLE providers ADD COLUMN check_in_day_offset_minutes INTEGER NOT NULL DEFAULT 480;
ALTER TABLE providers ADD COLUMN check_in_mode TEXT NOT NULL DEFAULT 'api';
-- Classify existing providers with the same rule CheckInMode::infer uses, so a
-- fresh install (which seeds through the config file) and an upgrade agree.
UPDATE providers SET check_in_mode = CASE
    WHEN sign_in_path IS NULL THEN 'observe_only'
    WHEN supports_check_in = 0 OR check_in_bugged = 1 THEN 'disabled'
    WHEN sign_in_path LIKE '%/api/%' THEN 'api'
    ELSE 'page_visit'
END;
-- Denormalised cache of the evidence ledger so account lists stay one query.
-- The ledger remains the single source of truth, and the repair command
-- recomputes this cache from it.
ALTER TABLE accounts ADD COLUMN last_confirmed_at TEXT;
ALTER TABLE accounts ADD COLUMN last_evidence_at TEXT;
UPDATE accounts SET
    last_confirmed_at = (
        SELECT MAX(c.recorded_at) FROM check_in_records c
        WHERE c.account_id = accounts.id AND c.source != 'legacy_estimate'
    ),
    last_evidence_at = (
        SELECT MAX(c.recorded_at) FROM check_in_records c WHERE c.account_id = accounts.id
    );
