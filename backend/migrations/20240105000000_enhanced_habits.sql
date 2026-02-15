-- Add time_period column to tasks table (nullable for backwards compatibility)
ALTER TABLE tasks ADD COLUMN time_period TEXT;

-- Create index for efficient queries on time_period
CREATE INDEX IF NOT EXISTS idx_tasks_time_period ON tasks(time_period);
