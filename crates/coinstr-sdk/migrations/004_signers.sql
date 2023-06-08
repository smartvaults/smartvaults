-- Database settings
PRAGMA user_version = 4; -- Schema version

CREATE TABLE IF NOT EXISTS signers (
    signer_id BLOB PRIMARY KEY NOT NULL,
    signer BLOB NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS signers_index ON signers(signer_id,signer);