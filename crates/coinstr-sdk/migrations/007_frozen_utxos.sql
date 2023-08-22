-- Database settings
PRAGMA user_version = 7; -- Schema version

CREATE TABLE IF NOT EXISTS frozen_utxos (
    utxo_hash BLOB PRIMARY KEY NOT NULL,
    policy_id BLOB NOT NULL,
    proposal_id BLOB
);