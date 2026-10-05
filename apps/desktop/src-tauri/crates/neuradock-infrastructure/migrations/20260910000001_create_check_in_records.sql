-- Rewards are independent of mutable balance snapshots. Amounts are stored in
-- cents and an account can have at most one record per UTC calendar day.
CREATE TABLE check_in_records (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    check_in_date TEXT NOT NULL,
    reward_cents INTEGER CHECK (reward_cents IS NULL OR reward_cents >= 0),
    source TEXT NOT NULL CHECK (source IN ('provider_response', 'provider_rule', 'confirmed_unknown', 'legacy_estimate', 'manual')),
    recorded_at TEXT NOT NULL,
    PRIMARY KEY (account_id, check_in_date)
);

-- Preserve old history as explicitly labelled estimates. Do not invent a
-- lifetime-sized reward for the first snapshot, or infer new rewards on reads.
WITH daily AS (
    SELECT account_id, DATE(recorded_at) AS day,
           MAX(total_quota) AS total_quota, MAX(recorded_at) AS recorded_at
    FROM balance_history
    GROUP BY account_id, DATE(recorded_at)
), increments AS (
    SELECT *, total_quota - LAG(total_quota) OVER (
        PARTITION BY account_id ORDER BY day
    ) AS increment
    FROM daily
)
INSERT INTO check_in_records (account_id, check_in_date, reward_cents, source, recorded_at)
SELECT account_id, day,
       CASE WHEN increment IS NULL THEN NULL ELSE CAST(ROUND(increment * 100) AS INTEGER) END,
       'legacy_estimate', recorded_at
FROM increments
WHERE (increment IS NULL OR increment > 0)
  AND EXISTS (SELECT 1 FROM accounts WHERE accounts.id = increments.account_id);
