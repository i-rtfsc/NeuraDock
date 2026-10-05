-- Drop phantom legacy days.
--
-- The legacy backfill inserted a row for every day whose quota did not go down,
-- which included the first snapshot of each account (its increment is NULL,
-- because there is nothing to compare against). That day has no increase to
-- infer a check-in from, so the row claims something nobody observed and quietly
-- adds a day to the account's streak.
--
-- A quota increase is deliberately treated as a check-in (a web check-in shows
-- up that way), but that reasoning does not apply to a day with no measurable
-- change at all.
DELETE FROM check_in_records
WHERE source = 'legacy_estimate' AND reward_cents IS NULL;

-- The account cache is derived from the ledger, so recompute it after pruning.
UPDATE accounts SET
    last_confirmed_at = (
        SELECT MAX(recorded_at) FROM check_in_records c
        WHERE c.account_id = accounts.id AND c.confidence IN ('confirmed', 'manual')
    ),
    last_evidence_at = (
        SELECT MAX(recorded_at) FROM check_in_records c WHERE c.account_id = accounts.id
    );
