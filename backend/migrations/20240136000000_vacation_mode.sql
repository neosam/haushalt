-- Add vacation mode fields to household_settings table
ALTER TABLE household_settings ADD COLUMN vacation_mode BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE household_settings ADD COLUMN vacation_start DATE;
ALTER TABLE household_settings ADD COLUMN vacation_end DATE;
