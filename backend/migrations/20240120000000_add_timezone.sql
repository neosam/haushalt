-- Add timezone support to household settings
ALTER TABLE household_settings ADD COLUMN timezone TEXT NOT NULL DEFAULT 'UTC';
