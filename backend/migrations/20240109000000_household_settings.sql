-- Household settings table for per-household configuration
-- Includes dark mode and customizable role labels

CREATE TABLE IF NOT EXISTS household_settings (
    household_id TEXT PRIMARY KEY NOT NULL REFERENCES households(id) ON DELETE CASCADE,
    dark_mode BOOLEAN NOT NULL DEFAULT FALSE,
    role_label_owner TEXT NOT NULL DEFAULT 'Owner',
    role_label_admin TEXT NOT NULL DEFAULT 'Admin',
    role_label_member TEXT NOT NULL DEFAULT 'Member',
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_household_settings_household ON household_settings(household_id);
