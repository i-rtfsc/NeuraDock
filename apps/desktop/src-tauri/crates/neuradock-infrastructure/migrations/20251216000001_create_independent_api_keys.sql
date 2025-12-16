-- Add Independent API Keys table
-- Stores user-provided API keys that are not tied to any provider account

CREATE TABLE IF NOT EXISTS independent_api_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,

    -- Provider information
    provider_type TEXT NOT NULL,  -- 'openai', 'anthropic', 'custom'
    custom_provider_name TEXT,    -- Only used when provider_type='custom'

    -- API credentials (encrypted)
    api_key TEXT NOT NULL,
    base_url TEXT NOT NULL,
    organization_id TEXT,         -- Optional, for OpenAI organization

    -- Metadata
    description TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,

    -- Timestamps
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_independent_keys_provider_type ON independent_api_keys(provider_type);
CREATE INDEX IF NOT EXISTS idx_independent_keys_active ON independent_api_keys(is_active);
