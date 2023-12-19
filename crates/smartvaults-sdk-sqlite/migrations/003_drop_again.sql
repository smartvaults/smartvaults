PRAGMA user_version = 3; -- Schema version

DROP TABLE IF EXISTS policies;
DROP TABLE IF EXISTS nostr_public_keys;
DROP TABLE IF EXISTS shared_keys;
DROP TABLE IF EXISTS pending_events;
DROP TABLE IF EXISTS proposals;
DROP TABLE IF EXISTS approved_proposals;
DROP TABLE IF EXISTS completed_proposals;
DROP TABLE IF EXISTS signers;
DROP TABLE IF EXISTS my_shared_signers;
DROP TABLE IF EXISTS shared_signers;
DROP TABLE IF EXISTS labels;
DROP TABLE IF EXISTS frozen_utxos;