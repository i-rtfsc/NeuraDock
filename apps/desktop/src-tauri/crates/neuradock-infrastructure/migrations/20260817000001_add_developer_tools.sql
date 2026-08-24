CREATE TABLE IF NOT EXISTS developer_tool_connections (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    service TEXT NOT NULL CHECK (service IN ('jira', 'gerrit')),
    base_url TEXT NOT NULL,
    deployment TEXT NOT NULL DEFAULT 'auto',
    auth_method TEXT NOT NULL,
    username TEXT,
    encrypted_secret TEXT,
    profile_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS developer_tool_saved_queries (
    id TEXT PRIMARY KEY NOT NULL,
    connection_id TEXT NOT NULL,
    name TEXT NOT NULL,
    query TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (connection_id) REFERENCES developer_tool_connections(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_developer_tool_queries_connection
    ON developer_tool_saved_queries(connection_id, updated_at DESC);
