-- Database settings
PRAGMA user_version = 6; -- Schema version

CREATE TABLE IF NOT EXISTS timechain (
    descriptor_hash BLOB PRIMARY KEY NOT NULL,
    data BLOB NOT NULL
);