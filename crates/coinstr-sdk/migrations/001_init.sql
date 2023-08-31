-- Database settings
PRAGMA encoding = "UTF-8";
PRAGMA journal_mode=WAL;
PRAGMA main.synchronous=NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA application_id = 1654008667;
PRAGMA user_version = 1; -- Schema version

-- Policies
CREATE TABLE IF NOT EXISTS policies (
    policy_id BLOB PRIMARY KEY NOT NULL,
    policy BLOB NOT NULL,
    last_sync BIGINT DEFAULT NULL
);

-- Nostr public keys
CREATE TABLE IF NOT EXISTS nostr_public_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    policy_id BLOB NOT NULL,
    public_key BLOB NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS nostr_public_keys_index ON nostr_public_keys(policy_id,public_key);

-- Shared keys
CREATE TABLE IF NOT EXISTS shared_keys (
    policy_id BLOB PRIMARY KEY NOT NULL,
    shared_key BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS pending_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event BLOB NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS pending_events_index ON pending_events(event);

-- Proposals
CREATE TABLE IF NOT EXISTS proposals (
    proposal_id BLOB PRIMARY KEY NOT NULL,
    policy_id BLOB NOT NULL,
    proposal BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS approved_proposals (
    approval_id BLOB PRIMARY KEY NOT NULL,
    proposal_id BLOB NOT NULL,
    public_key BLOB NOT NULL,
    approved_proposal BLOB NOT NULL,
    timestamp BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS completed_proposals (
    completed_proposal_id BLOB PRIMARY KEY NOT NULL,
    policy_id BLOB NOT NULL,
    completed_proposal BLOB NOT NULL
);

-- Relays
CREATE TABLE IF NOT EXISTS relays (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL,
    proxy BLOB DEFAULT NULL,
    last_sync BIGINT DEFAULT NULL,
    enabled BOOLEAN DEFAULT TRUE

);

CREATE UNIQUE INDEX IF NOT EXISTS relays_index ON relays(url);

-- Events
CREATE TABLE IF NOT EXISTS events (
    event_id BLOB PRIMARY KEY NOT NULL,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    event BLOB NOT NULL
);

-- Notifications
CREATE TABLE IF NOT EXISTS notifications (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id BLOB NOT NULL,
    notification BLOB NOT NULL,
    timestamp BIGINT NOT NULL,
    seen BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE UNIQUE INDEX IF NOT EXISTS notifications_index ON notifications(notification);

-- Contacts
CREATE TABLE IF NOT EXISTS contacts (
    public_key BLOB PRIMARY KEY NOT NULL
);

-- Metadata
CREATE TABLE IF NOT EXISTS metadata (
    public_key BLOB PRIMARY KEY NOT NULL,
    metadata BLOB NOT NULL,
    last_sync BIGINT DEFAULT NULL
);

-- Signers
CREATE TABLE IF NOT EXISTS signers (
    signer_id BLOB PRIMARY KEY NOT NULL,
    signer BLOB NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS signers_index ON signers(signer);

-- Shared signer that I have shared
CREATE TABLE IF NOT EXISTS my_shared_signers (
    shared_signer_id BLOB PRIMARY KEY NOT NULL,
    signer_id BLOB NOT NULL,
    public_key BLOB NOT NULL
);

-- Shared signers that I have received (others shared with me)
CREATE TABLE IF NOT EXISTS shared_signers (
    shared_signer_id BLOB PRIMARY KEY NOT NULL,
    owner_public_key BLOB NOT NULL,
    shared_signer BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS nostr_connect_sessions (
    app_public_key BLOB PRIMARY KEY NOT NULL,
    uri BLOB NOT NULL,
    timestamp BIGINT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS nostr_connect_sessions_index ON nostr_connect_sessions(uri);

CREATE TABLE IF NOT EXISTS nostr_connect_requests (
    event_id BLOB PRIMARY KEY NOT NULL,
    app_public_key BLOB NOT NULL,
    message BLOB NOT NULL,
    timestamp BIGINT NOT NULL,
    approved BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS labels (
    id BLOB PRIMARY KEY NOT NULL,
    policy_id BLOB NOT NULL,
    kind BLOB NOT NULL,
    label BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS timechain (
    descriptor_hash BLOB PRIMARY KEY NOT NULL,
    data BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS frozen_utxos (
    utxo_hash BLOB PRIMARY KEY NOT NULL,
    policy_id BLOB NOT NULL,
    proposal_id BLOB
);