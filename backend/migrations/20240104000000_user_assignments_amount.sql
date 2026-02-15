-- Migration: Convert user_rewards and user_punishments to amount-based model
-- Instead of multiple rows for each assignment, we have one row with an amount field

-- Create new user_rewards table with amount field
CREATE TABLE user_rewards_new (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id),
    reward_id TEXT NOT NULL REFERENCES rewards(id),
    household_id TEXT NOT NULL REFERENCES households(id),
    amount INTEGER NOT NULL DEFAULT 1,
    redeemed_amount INTEGER NOT NULL DEFAULT 0,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, reward_id, household_id)
);

-- Create new user_punishments table with amount field
CREATE TABLE user_punishments_new (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id),
    punishment_id TEXT NOT NULL REFERENCES punishments(id),
    household_id TEXT NOT NULL REFERENCES households(id),
    amount INTEGER NOT NULL DEFAULT 1,
    completed_amount INTEGER NOT NULL DEFAULT 0,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, punishment_id, household_id)
);

-- Migrate existing user_rewards data: aggregate rows into amounts
INSERT INTO user_rewards_new (id, user_id, reward_id, household_id, amount, redeemed_amount, updated_at)
SELECT
    MIN(id),
    user_id,
    reward_id,
    household_id,
    COUNT(*),
    SUM(CASE WHEN redeemed THEN 1 ELSE 0 END),
    MAX(assigned_at)
FROM user_rewards
GROUP BY user_id, reward_id, household_id;

-- Migrate existing user_punishments data: aggregate rows into amounts
INSERT INTO user_punishments_new (id, user_id, punishment_id, household_id, amount, completed_amount, updated_at)
SELECT
    MIN(id),
    user_id,
    punishment_id,
    household_id,
    COUNT(*),
    SUM(CASE WHEN completed THEN 1 ELSE 0 END),
    MAX(assigned_at)
FROM user_punishments
GROUP BY user_id, punishment_id, household_id;

-- Drop old tables
DROP TABLE user_rewards;
DROP TABLE user_punishments;

-- Rename new tables
ALTER TABLE user_rewards_new RENAME TO user_rewards;
ALTER TABLE user_punishments_new RENAME TO user_punishments;

-- Recreate indexes
CREATE INDEX idx_user_rewards_user ON user_rewards(user_id);
CREATE INDEX idx_user_rewards_household ON user_rewards(household_id);
CREATE INDEX idx_user_punishments_user ON user_punishments(user_id);
CREATE INDEX idx_user_punishments_household ON user_punishments(household_id);
