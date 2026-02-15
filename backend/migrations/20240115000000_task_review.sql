-- Add requires_review to tasks table
ALTER TABLE tasks ADD COLUMN requires_review BOOLEAN NOT NULL DEFAULT FALSE;

-- Add status to task_completions table (approved = default, pending = awaiting review)
ALTER TABLE task_completions ADD COLUMN status TEXT NOT NULL DEFAULT 'approved';

-- Index for finding pending completions efficiently
CREATE INDEX IF NOT EXISTS idx_completions_status ON task_completions(status);
