-- Add hierarchy_type column to household_settings
-- Values: 'equals', 'organized', 'hierarchy'
-- Default: 'organized' (matches current behavior)

ALTER TABLE household_settings ADD COLUMN hierarchy_type TEXT NOT NULL DEFAULT 'organized';
