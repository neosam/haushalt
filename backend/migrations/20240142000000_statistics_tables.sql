-- Weekly statistics per household member
CREATE TABLE weekly_statistics (
    id TEXT PRIMARY KEY NOT NULL,
    household_id TEXT NOT NULL REFERENCES households(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    week_start DATE NOT NULL,
    week_end DATE NOT NULL,
    total_expected INTEGER NOT NULL,
    total_completed INTEGER NOT NULL,
    completion_rate REAL NOT NULL,
    calculated_at DATETIME NOT NULL,
    UNIQUE(household_id, user_id, week_start)
);

CREATE INDEX idx_weekly_stats_household ON weekly_statistics(household_id);
CREATE INDEX idx_weekly_stats_user ON weekly_statistics(user_id);
CREATE INDEX idx_weekly_stats_lookup ON weekly_statistics(household_id, week_start);

-- Per-task breakdown for weekly statistics
CREATE TABLE weekly_statistics_tasks (
    id TEXT PRIMARY KEY NOT NULL,
    weekly_statistics_id TEXT NOT NULL REFERENCES weekly_statistics(id) ON DELETE CASCADE,
    task_id TEXT NOT NULL,
    task_title TEXT NOT NULL,
    expected INTEGER NOT NULL,
    completed INTEGER NOT NULL,
    completion_rate REAL NOT NULL
);

CREATE INDEX idx_weekly_stats_tasks_parent ON weekly_statistics_tasks(weekly_statistics_id);

-- Monthly statistics per household member
CREATE TABLE monthly_statistics (
    id TEXT PRIMARY KEY NOT NULL,
    household_id TEXT NOT NULL REFERENCES households(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    month DATE NOT NULL,
    total_expected INTEGER NOT NULL,
    total_completed INTEGER NOT NULL,
    completion_rate REAL NOT NULL,
    calculated_at DATETIME NOT NULL,
    UNIQUE(household_id, user_id, month)
);

CREATE INDEX idx_monthly_stats_household ON monthly_statistics(household_id);
CREATE INDEX idx_monthly_stats_user ON monthly_statistics(user_id);
CREATE INDEX idx_monthly_stats_lookup ON monthly_statistics(household_id, month);

-- Per-task breakdown for monthly statistics
CREATE TABLE monthly_statistics_tasks (
    id TEXT PRIMARY KEY NOT NULL,
    monthly_statistics_id TEXT NOT NULL REFERENCES monthly_statistics(id) ON DELETE CASCADE,
    task_id TEXT NOT NULL,
    task_title TEXT NOT NULL,
    expected INTEGER NOT NULL,
    completed INTEGER NOT NULL,
    completion_rate REAL NOT NULL
);

CREATE INDEX idx_monthly_stats_tasks_parent ON monthly_statistics_tasks(monthly_statistics_id);
