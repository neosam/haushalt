-- Add archived column to tasks table
ALTER TABLE tasks ADD COLUMN archived BOOLEAN NOT NULL DEFAULT 0;

-- Index for filtering archived tasks
CREATE INDEX IF NOT EXISTS idx_tasks_archived ON tasks(archived);
