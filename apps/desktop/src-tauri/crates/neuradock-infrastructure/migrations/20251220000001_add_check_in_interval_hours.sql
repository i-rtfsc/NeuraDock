-- Add configurable check-in interval hours to accounts table
ALTER TABLE accounts ADD COLUMN check_in_interval_hours INTEGER NOT NULL DEFAULT 23;

-- Ensure existing accounts have the default value
UPDATE accounts SET check_in_interval_hours = 23 WHERE check_in_interval_hours IS NULL;
