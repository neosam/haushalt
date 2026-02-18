-- Add paused field to tasks table
ALTER TABLE tasks ADD COLUMN paused BOOLEAN NOT NULL DEFAULT 0;

-- Index for efficient filtering
CREATE INDEX IF NOT EXISTS idx_tasks_paused ON tasks(paused);
