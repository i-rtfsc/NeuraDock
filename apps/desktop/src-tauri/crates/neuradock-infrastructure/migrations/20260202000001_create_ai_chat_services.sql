-- AI Chat Services table for embedded chat webviews
CREATE TABLE IF NOT EXISTS ai_chat_services (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    icon TEXT,
    is_builtin INTEGER NOT NULL DEFAULT 0,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Create index for sorting
CREATE INDEX IF NOT EXISTS idx_ai_chat_services_sort ON ai_chat_services(sort_order, name);
