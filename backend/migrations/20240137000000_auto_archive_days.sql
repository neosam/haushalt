-- Add auto_archive_days setting for automatic archival of completed one-time and custom tasks
-- Default: 7 days, NULL or 0 disables auto-archiving
ALTER TABLE household_settings ADD COLUMN auto_archive_days INTEGER DEFAULT 7;
