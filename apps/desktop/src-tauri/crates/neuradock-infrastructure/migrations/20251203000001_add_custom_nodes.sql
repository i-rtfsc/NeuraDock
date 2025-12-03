-- Custom provider nodes table
CREATE TABLE IF NOT EXISTS custom_provider_nodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id TEXT NOT NULL,
    name TEXT NOT NULL,
    base_url TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(provider_id, base_url)
);

CREATE INDEX idx_custom_nodes_provider ON custom_provider_nodes(provider_id);
