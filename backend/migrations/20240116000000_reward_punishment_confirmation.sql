-- Add requires_confirmation to rewards table
ALTER TABLE rewards ADD COLUMN requires_confirmation BOOLEAN NOT NULL DEFAULT FALSE;

-- Add requires_confirmation to punishments table
ALTER TABLE punishments ADD COLUMN requires_confirmation BOOLEAN NOT NULL DEFAULT FALSE;

-- Add pending tracking to user_rewards
ALTER TABLE user_rewards ADD COLUMN pending_redemption INTEGER NOT NULL DEFAULT 0;

-- Add pending tracking to user_punishments
ALTER TABLE user_punishments ADD COLUMN pending_completion INTEGER NOT NULL DEFAULT 0;

-- Indexes for efficient pending queries
CREATE INDEX IF NOT EXISTS idx_user_rewards_pending ON user_rewards(pending_redemption) WHERE pending_redemption > 0;
CREATE INDEX IF NOT EXISTS idx_user_punishments_pending ON user_punishments(pending_completion) WHERE pending_completion > 0;
