-- Add allow_exceed_target column to tasks table
-- When true (default), users can track completions beyond the target count
-- When false, the complete button is disabled once target is reached
ALTER TABLE tasks ADD COLUMN allow_exceed_target BOOLEAN NOT NULL DEFAULT 1;
