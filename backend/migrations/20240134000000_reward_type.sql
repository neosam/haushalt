-- Add reward_type column to rewards table for extensible reward types
ALTER TABLE rewards ADD COLUMN reward_type TEXT NOT NULL DEFAULT 'standard';

-- Create reward_options table for linking random choice rewards to their options
CREATE TABLE reward_options (
    id TEXT PRIMARY KEY NOT NULL,
    parent_reward_id TEXT NOT NULL REFERENCES rewards(id) ON DELETE CASCADE,
    option_reward_id TEXT NOT NULL REFERENCES rewards(id) ON DELETE CASCADE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(parent_reward_id, option_reward_id)
);

CREATE INDEX idx_reward_options_parent ON reward_options(parent_reward_id);
