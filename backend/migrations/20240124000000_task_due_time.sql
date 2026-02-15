-- Add due_time field to tasks
-- Format: "HH:MM" or NULL (defaults to 23:59 - end of day)
ALTER TABLE tasks ADD COLUMN due_time TEXT;
