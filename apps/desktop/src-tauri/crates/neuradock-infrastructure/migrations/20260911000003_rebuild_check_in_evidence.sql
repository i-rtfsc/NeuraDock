-- Rebuild the evidence ledger with an explicit confidence and a quota-day
-- bucket, and re-bucket existing rows.
--
-- Three things need to change at once and SQLite cannot ALTER a CHECK or rename
-- a semantic, so the table is rebuilt:
--   1. `check_in_date` becomes `quota_day`: the provider's reset day, computed
--      from the stored UTC instant plus the provider's offset, instead of a UTC
--      calendar day. Re-bucketing matters for check-ins recorded between 16:00
--      and 24:00 UTC, which belong to the next local day for CN providers.
--   2. `confidence` distinguishes a provider-confirmed reward from one inferred
--      from a quota increase, and from a manual correction.
--   3. `source` must admit `balance_delta` (the runtime inference path).
--
-- Re-bucketing can map two rows onto the same day, so the copy keeps one row per
-- (account, quota_day) by the same precedence the runtime UPSERT uses:
-- manual > provider_response > provider_rule > confirmed_unknown >
-- balance_delta > legacy_estimate, then the larger reward, then the later
-- instant.
CREATE TABLE check_in_records_v2 (
    account_id   TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    quota_day    TEXT NOT NULL,
    reward_cents INTEGER CHECK (reward_cents IS NULL OR reward_cents >= 0),
    source       TEXT NOT NULL CHECK (source IN (
                     'provider_response', 'provider_rule', 'confirmed_unknown',
                     'legacy_estimate', 'manual', 'balance_delta')),
    confidence   TEXT NOT NULL CHECK (confidence IN ('confirmed', 'inferred', 'manual')),
    recorded_at  TEXT NOT NULL,
    attempt_id   INTEGER REFERENCES check_in_attempts(id) ON DELETE SET NULL,
    PRIMARY KEY (account_id, quota_day)
);
INSERT INTO check_in_records_v2 (account_id, quota_day, reward_cents, source, confidence, recorded_at, attempt_id)
SELECT account_id, quota_day, reward_cents, source, confidence, recorded_at, NULL
FROM (
    SELECT c.account_id AS account_id,
           COALESCE(
               date(c.recorded_at, printf('%+d minutes', COALESCE(p.check_in_day_offset_minutes, 480))),
               c.check_in_date
           ) AS quota_day,
           c.reward_cents AS reward_cents,
           c.source AS source,
           CASE c.source
               WHEN 'manual' THEN 'manual'
               WHEN 'legacy_estimate' THEN 'inferred'
               WHEN 'balance_delta' THEN 'inferred'
               ELSE 'confirmed'
           END AS confidence,
           c.recorded_at AS recorded_at,
           ROW_NUMBER() OVER (
               PARTITION BY c.account_id, COALESCE(
                   date(c.recorded_at, printf('%+d minutes', COALESCE(p.check_in_day_offset_minutes, 480))),
                   c.check_in_date
               )
               ORDER BY CASE c.source
                            WHEN 'manual' THEN 0
                            WHEN 'provider_response' THEN 1
                            WHEN 'provider_rule' THEN 2
                            WHEN 'confirmed_unknown' THEN 3
                            WHEN 'balance_delta' THEN 4
                            ELSE 5
                        END,
                        c.reward_cents DESC,
                        c.recorded_at DESC
           ) AS rn
    FROM check_in_records c
    LEFT JOIN accounts a ON a.id = c.account_id
    LEFT JOIN providers p ON p.id = a.provider_id
) ranked
WHERE rn = 1;
DROP TABLE check_in_records;
ALTER TABLE check_in_records_v2 RENAME TO check_in_records;
