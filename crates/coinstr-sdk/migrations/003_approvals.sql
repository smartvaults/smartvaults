-- Database settings
PRAGMA user_version = 3; -- Schema version

CREATE TABLE IF NOT EXISTS approved_proposals (
    approval_id BLOB PRIMARY KEY NOT NULL,
    proposal_id BLOB NOT NULL,
    public_key BLOB NOT NULL,
    approved_proposal BLOB NOT NULL,
    timestamp BIGINT NOT NULL
);