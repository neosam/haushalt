-- Convert tasks to habits: allow multiple completions per period

-- Add target_count to tasks (default 1 for backward compatibility)
ALTER TABLE tasks ADD COLUMN target_count INTEGER NOT NULL DEFAULT 1;

-- Remove the unique constraint to allow multiple completions per day
DROP INDEX IF EXISTS idx_completions_unique;

-- Create new index for efficient completion counting
CREATE INDEX IF NOT EXISTS idx_completions_task_user_date
    ON task_completions(task_id, user_id, due_date);
