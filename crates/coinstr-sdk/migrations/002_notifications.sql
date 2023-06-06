-- Database settings
PRAGMA user_version = 2; -- Schema version

CREATE TABLE IF NOT EXISTS notifications (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    notification BLOB NOT NULL,
    timestamp BIGINT NOT NULL,
    seen BOOLEAN NOT NULL DEFAULT FALSE
);