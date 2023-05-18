-- Database settings
PRAGMA encoding = "UTF-8";
PRAGMA journal_mode=WAL;
PRAGMA main.synchronous=NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA application_id = 1654008667;
PRAGMA user_version = 1; -- Schema version

-- Policies keys Table
CREATE TABLE IF NOT EXISTS policies (
    policy_id BLOB PRIMARY KEY NOT NULL,
    policy BLOB NOT NULL,
    last_sync BIGINT DEFAULT NULL
);

-- Nostr public keys Table
CREATE TABLE IF NOT EXISTS nostr_public_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    policy_id BLOB NOT NULL,
    public_key BLOB NOT NULL
);

-- Nostr public keys Indexes
CREATE UNIQUE INDEX IF NOT EXISTS nostr_public_keys_index ON nostr_public_keys(policy_id,public_key);

-- Shared keys Table
CREATE TABLE IF NOT EXISTS shared_keys (
    policy_id BLOB PRIMARY KEY NOT NULL,
    shared_key BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS proposals (
    proposal_id BLOB PRIMARY KEY NOT NULL,
    policy_id BLOB NOT NULL,
    proposal BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS completed_proposals (
    completed_proposal_id BLOB PRIMARY KEY NOT NULL,
    policy_id BLOB NOT NULL,
    completed_proposal BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS relays (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL,
    last_sync BIGINT DEFAULT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS relays_index ON relays(url);

CREATE TABLE IF NOT EXISTS events (
    event_id BLOB PRIMARY KEY NOT NULL,
    event BLOB NOT NULL
);
