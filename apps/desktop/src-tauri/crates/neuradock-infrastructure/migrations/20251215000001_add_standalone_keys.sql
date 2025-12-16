-- Add Standalone API Keys table for manually added keys
-- This stores user-provided API keys that are not tied to any provider account

CREATE TABLE IF NOT EXISTS standalone_keys (
    id TEXT PRIMARY KEY,  -- UUID
    name TEXT NOT NULL,   -- User-friendly name (e.g., "OpenAI Pro", "Anthropic API")

    -- Provider information
    provider_type TEXT NOT NULL,  -- 'openai', 'anthropic', 'custom'
    base_url TEXT,  -- Optional custom base URL

    -- API Key (encrypted)
    api_key TEXT NOT NULL,

    -- Optional metadata
    description TEXT,

    -- Status
    enabled BOOLEAN NOT NULL DEFAULT 1,

    -- Timestamps
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_standalone_keys_provider_type ON standalone_keys(provider_type);
CREATE INDEX IF NOT EXISTS idx_standalone_keys_enabled ON standalone_keys(enabled) WHERE enabled = 1;
