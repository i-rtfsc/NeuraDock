-- Drop the deprecated standalone_keys table
-- This table was replaced by independent_api_keys in migration 20251216000001
-- The new table has better schema design and naming

DROP TABLE IF EXISTS standalone_keys;
