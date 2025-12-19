-- Fix missing or invalid created_at values in providers table
-- Convert SQLite CURRENT_TIMESTAMP format to RFC3339 format for Rust parsing

-- Convert SQLite format (YYYY-MM-DD HH:MM:SS) to RFC3339 format (YYYY-MM-DDTHH:MM:SSZ)
UPDATE providers
SET created_at = strftime('%Y-%m-%dT%H:%M:%SZ', created_at)
WHERE created_at IS NOT NULL
  AND created_at != ''
  AND created_at NOT LIKE '%T%Z';  -- Skip already converted timestamps

-- Update NULL or empty created_at values with current timestamp in RFC3339 format
UPDATE providers
SET created_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
WHERE created_at IS NULL OR created_at = '' OR length(trim(created_at)) = 0;
