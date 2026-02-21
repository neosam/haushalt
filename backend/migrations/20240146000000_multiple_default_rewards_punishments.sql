-- Create junction tables for multiple default rewards/punishments

CREATE TABLE household_default_rewards (
    household_id TEXT NOT NULL REFERENCES households(id) ON DELETE CASCADE,
    reward_id TEXT NOT NULL REFERENCES rewards(id) ON DELETE CASCADE,
    amount INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (household_id, reward_id)
);

CREATE TABLE household_default_punishments (
    household_id TEXT NOT NULL REFERENCES households(id) ON DELETE CASCADE,
    punishment_id TEXT NOT NULL REFERENCES punishments(id) ON DELETE CASCADE,
    amount INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (household_id, punishment_id)
);

-- Migrate existing single defaults to new tables (if set)
INSERT INTO household_default_rewards (household_id, reward_id, amount)
SELECT household_id, default_reward_id, 1
FROM household_settings
WHERE default_reward_id IS NOT NULL;

INSERT INTO household_default_punishments (household_id, punishment_id, amount)
SELECT household_id, default_punishment_id, 1
FROM household_settings
WHERE default_punishment_id IS NOT NULL;

-- Remove old columns by recreating the table (SQLite limitation)
-- Step 1: Create new table without the old columns
CREATE TABLE household_settings_new (
    household_id TEXT PRIMARY KEY NOT NULL REFERENCES households(id),
    dark_mode BOOLEAN NOT NULL DEFAULT 0,
    role_label_owner TEXT NOT NULL DEFAULT 'Owner',
    role_label_admin TEXT NOT NULL DEFAULT 'Admin',
    role_label_member TEXT NOT NULL DEFAULT 'Member',
    hierarchy_type TEXT NOT NULL DEFAULT 'organized',
    timezone TEXT NOT NULL DEFAULT 'UTC',
    rewards_enabled BOOLEAN NOT NULL DEFAULT 0,
    punishments_enabled BOOLEAN NOT NULL DEFAULT 0,
    chat_enabled BOOLEAN NOT NULL DEFAULT 0,
    vacation_mode BOOLEAN NOT NULL DEFAULT 0,
    vacation_start DATE,
    vacation_end DATE,
    auto_archive_days INTEGER DEFAULT 7,
    allow_task_suggestions BOOLEAN NOT NULL DEFAULT 1,
    week_start_day INTEGER NOT NULL DEFAULT 0,
    default_points_reward INTEGER,
    default_points_penalty INTEGER,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Step 2: Copy data from old table to new table
INSERT INTO household_settings_new (
    household_id, dark_mode, role_label_owner, role_label_admin, role_label_member,
    hierarchy_type, timezone, rewards_enabled, punishments_enabled, chat_enabled,
    vacation_mode, vacation_start, vacation_end, auto_archive_days, allow_task_suggestions,
    week_start_day, default_points_reward, default_points_penalty, updated_at
)
SELECT
    household_id, dark_mode, role_label_owner, role_label_admin, role_label_member,
    hierarchy_type, timezone, rewards_enabled, punishments_enabled, chat_enabled,
    vacation_mode, vacation_start, vacation_end, auto_archive_days, allow_task_suggestions,
    week_start_day, default_points_reward, default_points_penalty, updated_at
FROM household_settings;

-- Step 3: Drop old table and rename new table
DROP TABLE household_settings;
ALTER TABLE household_settings_new RENAME TO household_settings;
