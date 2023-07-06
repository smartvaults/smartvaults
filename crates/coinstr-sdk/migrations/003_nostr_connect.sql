-- Database settings
PRAGMA user_version = 3; -- Schema version

CREATE TABLE IF NOT EXISTS nostr_connect_sessions (
    app_public_key BLOB PRIMARY KEY NOT NULL,
    uri BLOB NOT NULL,
    timestamp BIGINT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS nostr_connect_sessions_index ON nostr_connect_sessions(uri);

CREATE TABLE IF NOT EXISTS nostr_connect_requests (
    event_id BLOB PRIMARY KEY AUTOINCREMENT,
    app_public_key BLOB NOT NULL,
    message BLOB NOT NULL,
    timestamp BIGINT NOT NULL,
    approved BOOLEAN NOT NULL DEFAUL FALSE
);

-- CREATE UNIQUE INDEX IF NOT EXISTS nostr_connect_requests_index ON nostr_connect_requests(app_public_key,message);