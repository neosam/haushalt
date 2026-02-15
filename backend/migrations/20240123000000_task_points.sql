-- Add direct points fields to tasks
-- points_reward: Points awarded when task is completed
-- points_penalty: Points deducted when task is missed

ALTER TABLE tasks ADD COLUMN points_reward INTEGER;
ALTER TABLE tasks ADD COLUMN points_penalty INTEGER;
