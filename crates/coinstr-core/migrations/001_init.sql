-- Database settings
PRAGMA encoding = "UTF-8";
PRAGMA journal_mode=WAL;
PRAGMA main.synchronous=NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA application_id = 1654008667;
PRAGMA user_version = 1; -- Schema version

-- Policies keys Table
CREATE TABLE IF NOT EXISTS policies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    policy_id BLOB NOT NULL,
    policy BLOB NOT NULL,
    last_sync BIGINT DEFAULT NULL
);

-- Policies keys Indexes
CREATE UNIQUE INDEX IF NOT EXISTS policy_id_index ON policies(policy_id);

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
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    policy_id BLOB NOT NULL,
    shared_key BLOB NOT NULL
);

-- Shared keys Indexes
CREATE UNIQUE INDEX IF NOT EXISTS shared_key_policy_id_index ON shared_keys(policy_id);

CREATE TABLE IF NOT EXISTS proposals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    proposal_id BLOB NOT NULL,
    policy_id BLOB NOT NULL,
    proposal BLOB NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS proposal_index ON proposals(proposal_id,policy_id);

CREATE TABLE IF NOT EXISTS completed_proposals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    completed_proposal_id BLOB NOT NULL,
    policy_id BLOB NOT NULL,
    completed_proposal BLOB NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS completed_proposal_index ON completed_proposals(completed_proposal_id,policy_id);
