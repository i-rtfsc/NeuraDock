-- Add provider check-in capability flags
ALTER TABLE providers ADD COLUMN supports_check_in BOOLEAN NOT NULL DEFAULT 1;
ALTER TABLE providers ADD COLUMN check_in_bugged BOOLEAN NOT NULL DEFAULT 0;

-- Ensure existing providers have sensible defaults
UPDATE providers SET supports_check_in = 1 WHERE supports_check_in IS NULL;
UPDATE providers SET check_in_bugged = 1 WHERE id = 'agentrouter';

