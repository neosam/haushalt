-- Add week_start_day setting to household_settings
-- 0=Monday, 1=Tuesday, 2=Wednesday, 3=Thursday, 4=Friday, 5=Saturday, 6=Sunday
ALTER TABLE household_settings ADD COLUMN week_start_day INTEGER NOT NULL DEFAULT 0;
