-- Initial migration for household management system

-- Users table
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY NOT NULL,
    username TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT,
    oidc_subject TEXT,
    oidc_provider TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_oidc ON users(oidc_provider, oidc_subject);

-- Households table
CREATE TABLE IF NOT EXISTS households (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    owner_id TEXT NOT NULL REFERENCES users(id),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_households_owner ON households(owner_id);

-- Household memberships table
CREATE TABLE IF NOT EXISTS household_memberships (
    id TEXT PRIMARY KEY NOT NULL,
    household_id TEXT NOT NULL REFERENCES households(id),
    user_id TEXT NOT NULL REFERENCES users(id),
    role TEXT NOT NULL DEFAULT 'member' CHECK(role IN ('owner', 'admin', 'member')),
    points INTEGER NOT NULL DEFAULT 0,
    joined_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(household_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_memberships_household ON household_memberships(household_id);
CREATE INDEX IF NOT EXISTS idx_memberships_user ON household_memberships(user_id);
CREATE INDEX IF NOT EXISTS idx_memberships_points ON household_memberships(household_id, points DESC);

-- Tasks table
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY NOT NULL,
    household_id TEXT NOT NULL REFERENCES households(id),
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    recurrence_type TEXT NOT NULL DEFAULT 'daily' CHECK(recurrence_type IN ('daily', 'weekly', 'monthly', 'weekdays', 'custom')),
    recurrence_value TEXT,
    assigned_user_id TEXT REFERENCES users(id),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_tasks_household ON tasks(household_id);
CREATE INDEX IF NOT EXISTS idx_tasks_assigned ON tasks(assigned_user_id);

-- Task completions table
CREATE TABLE IF NOT EXISTS task_completions (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    user_id TEXT NOT NULL REFERENCES users(id),
    completed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    due_date DATE NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_completions_task ON task_completions(task_id);
CREATE INDEX IF NOT EXISTS idx_completions_user ON task_completions(user_id);
CREATE INDEX IF NOT EXISTS idx_completions_date ON task_completions(due_date);
CREATE UNIQUE INDEX IF NOT EXISTS idx_completions_unique ON task_completions(task_id, user_id, due_date);

-- Point conditions table
CREATE TABLE IF NOT EXISTS point_conditions (
    id TEXT PRIMARY KEY NOT NULL,
    household_id TEXT NOT NULL REFERENCES households(id),
    name TEXT NOT NULL,
    condition_type TEXT NOT NULL CHECK(condition_type IN ('task_complete', 'task_missed', 'streak', 'streak_broken')),
    points_value INTEGER NOT NULL,
    streak_threshold INTEGER,
    multiplier REAL,
    task_id TEXT REFERENCES tasks(id),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_conditions_household ON point_conditions(household_id);
CREATE INDEX IF NOT EXISTS idx_conditions_task ON point_conditions(task_id);

-- Rewards table
CREATE TABLE IF NOT EXISTS rewards (
    id TEXT PRIMARY KEY NOT NULL,
    household_id TEXT NOT NULL REFERENCES households(id),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    point_cost INTEGER,
    is_purchasable BOOLEAN NOT NULL DEFAULT FALSE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_rewards_household ON rewards(household_id);

-- Punishments table
CREATE TABLE IF NOT EXISTS punishments (
    id TEXT PRIMARY KEY NOT NULL,
    household_id TEXT NOT NULL REFERENCES households(id),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_punishments_household ON punishments(household_id);

-- Task-reward associations
CREATE TABLE IF NOT EXISTS task_rewards (
    task_id TEXT NOT NULL REFERENCES tasks(id),
    reward_id TEXT NOT NULL REFERENCES rewards(id),
    PRIMARY KEY (task_id, reward_id)
);

-- Task-punishment associations
CREATE TABLE IF NOT EXISTS task_punishments (
    task_id TEXT NOT NULL REFERENCES tasks(id),
    punishment_id TEXT NOT NULL REFERENCES punishments(id),
    PRIMARY KEY (task_id, punishment_id)
);

-- User rewards (assigned or purchased)
CREATE TABLE IF NOT EXISTS user_rewards (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id),
    reward_id TEXT NOT NULL REFERENCES rewards(id),
    household_id TEXT NOT NULL REFERENCES households(id),
    assigned_by TEXT REFERENCES users(id),
    is_purchased BOOLEAN NOT NULL DEFAULT FALSE,
    redeemed BOOLEAN NOT NULL DEFAULT FALSE,
    assigned_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_user_rewards_user ON user_rewards(user_id);
CREATE INDEX IF NOT EXISTS idx_user_rewards_household ON user_rewards(household_id);
CREATE INDEX IF NOT EXISTS idx_user_rewards_redeemed ON user_rewards(redeemed);

-- User punishments (assigned)
CREATE TABLE IF NOT EXISTS user_punishments (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id),
    punishment_id TEXT NOT NULL REFERENCES punishments(id),
    household_id TEXT NOT NULL REFERENCES households(id),
    assigned_by TEXT NOT NULL REFERENCES users(id),
    task_completion_id TEXT REFERENCES task_completions(id),
    completed BOOLEAN NOT NULL DEFAULT FALSE,
    assigned_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_user_punishments_user ON user_punishments(user_id);
CREATE INDEX IF NOT EXISTS idx_user_punishments_household ON user_punishments(household_id);
CREATE INDEX IF NOT EXISTS idx_user_punishments_completed ON user_punishments(completed);
