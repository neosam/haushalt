-- Add assignee_cannot_uncomplete column to tasks table
-- When false (default), behaviour is unchanged
-- When true, every household member may check the task off (including the assigned user) and
-- every member's completions count toward target_count, but the assigned user may NOT undo a
-- completion - someone else from the household has to clear it
ALTER TABLE tasks ADD COLUMN assignee_cannot_uncomplete BOOLEAN NOT NULL DEFAULT 0;
