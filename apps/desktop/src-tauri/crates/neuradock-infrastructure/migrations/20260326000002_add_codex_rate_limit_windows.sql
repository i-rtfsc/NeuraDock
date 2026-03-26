ALTER TABLE codex_accounts ADD COLUMN is_unlimited INTEGER;
ALTER TABLE codex_accounts ADD COLUMN primary_used_percent REAL;
ALTER TABLE codex_accounts ADD COLUMN primary_window_minutes INTEGER;
ALTER TABLE codex_accounts ADD COLUMN primary_resets_at TEXT;
ALTER TABLE codex_accounts ADD COLUMN secondary_used_percent REAL;
ALTER TABLE codex_accounts ADD COLUMN secondary_window_minutes INTEGER;
ALTER TABLE codex_accounts ADD COLUMN secondary_resets_at TEXT;
