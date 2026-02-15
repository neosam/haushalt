-- User-specific dashboard task whitelist
-- Each user can choose which tasks appear on their personal dashboard

CREATE TABLE user_dashboard_tasks (
    user_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, task_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- Index for faster lookups by user
CREATE INDEX idx_user_dashboard_tasks_user ON user_dashboard_tasks(user_id);
