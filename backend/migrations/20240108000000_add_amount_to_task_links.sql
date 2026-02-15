-- Add amount column to task_rewards and task_punishments tables
-- Amount represents how many times a reward/punishment is applied when the task is completed/missed

ALTER TABLE task_rewards ADD COLUMN amount INTEGER NOT NULL DEFAULT 1;
ALTER TABLE task_punishments ADD COLUMN amount INTEGER NOT NULL DEFAULT 1;
