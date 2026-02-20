-- Add setting to control whether members can suggest tasks
ALTER TABLE household_settings ADD COLUMN allow_task_suggestions BOOLEAN NOT NULL DEFAULT 1;
