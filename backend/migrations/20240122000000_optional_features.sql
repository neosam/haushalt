-- Add optional feature flags to household_settings
-- All features are disabled by default
ALTER TABLE household_settings ADD COLUMN rewards_enabled BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE household_settings ADD COLUMN punishments_enabled BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE household_settings ADD COLUMN chat_enabled BOOLEAN NOT NULL DEFAULT FALSE;
