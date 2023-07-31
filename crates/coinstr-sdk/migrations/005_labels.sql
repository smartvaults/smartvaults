-- Database settings
PRAGMA user_version = 5; -- Schema version

CREATE TABLE IF NOT EXISTS labels (
    id BLOB PRIMARY KEY NOT NULL,
    policy_id BLOB NOT NULL,
    key BLOB NOT NULL,
    label BLOB NOT NULL
);