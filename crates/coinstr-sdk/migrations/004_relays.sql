-- Database settings
PRAGMA user_version = 4; -- Schema version

ALTER TABLE relays ADD COLUMN enabled BOOLEAN DEFAULT TRUE;
ALTER TABLE relays ADD COLUMN proxy BLOB DEFAULT NULL;