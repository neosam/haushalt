-- Add task default settings to household_settings
ALTER TABLE household_settings ADD COLUMN default_points_reward INTEGER;
ALTER TABLE household_settings ADD COLUMN default_points_penalty INTEGER;
ALTER TABLE household_settings ADD COLUMN default_reward_id TEXT;
ALTER TABLE household_settings ADD COLUMN default_punishment_id TEXT;
