-- Add habit_type column to tasks table
-- 'good' = normal habit (completion rewards, missed punishes)
-- 'bad' = inverted habit (completion punishes, missed rewards)

ALTER TABLE tasks ADD COLUMN habit_type TEXT NOT NULL DEFAULT 'good';
