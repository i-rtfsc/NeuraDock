-- Create reminder_settings table for missed check-in reminders.
--
-- Singleton, like proxy_settings: a missed check-in reminder answers
-- "what do I still owe today", which is a property of the day rather
-- than of any one account. Per-account schedules would fan one
-- forgotten morning out into four separate pings.
CREATE TABLE IF NOT EXISTS reminder_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    enabled BOOLEAN NOT NULL DEFAULT 0,
    -- Comma-separated local-time HH:MM points. The number of entries IS
    -- the number of reminders per day, so there is deliberately no separate
    -- count column that could contradict this list.
    times TEXT NOT NULL DEFAULT '14:00,15:00,20:00',
    -- Comma-separated subset of: app, feishu
    channels TEXT NOT NULL DEFAULT 'app,feishu',
    -- Skip the remaining time points once every account has checked in.
    stop_when_all_done BOOLEAN NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Default to afternoon/evening points, deliberately after the morning
-- auto check-in window. Disabled until the user opts in.
INSERT OR IGNORE INTO reminder_settings
    (id, enabled, times, channels, stop_when_all_done)
VALUES (1, 0, '14:00,15:00,20:00', 'app,feishu', 1);
