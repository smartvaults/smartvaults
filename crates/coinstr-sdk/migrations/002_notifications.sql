-- Database settings
PRAGMA user_version = 2; -- Schema version

ALTER TABLE notifications ADD COLUMN event_id BLOB;
DELETE FROM notifications;
