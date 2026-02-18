-- Convert is_random_choice boolean to punishment_type text column
-- This allows for extensible punishment types in the future

-- Add new punishment_type column with default value
ALTER TABLE punishments ADD COLUMN punishment_type TEXT NOT NULL DEFAULT 'standard';

-- Migrate existing data: is_random_choice = true → 'random_choice'
UPDATE punishments SET punishment_type = 'random_choice' WHERE is_random_choice = 1;

-- Drop the old is_random_choice column (SQLite 3.35+)
ALTER TABLE punishments DROP COLUMN is_random_choice;
