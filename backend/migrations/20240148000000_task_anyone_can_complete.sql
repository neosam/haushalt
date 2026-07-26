-- Add anyone_can_complete column to tasks table
-- When false (default), only the assigned user may complete/uncomplete the task
-- When true, any household member may complete/uncomplete it, and every member's
-- completions count toward target_count
ALTER TABLE tasks ADD COLUMN anyone_can_complete BOOLEAN NOT NULL DEFAULT 0;
