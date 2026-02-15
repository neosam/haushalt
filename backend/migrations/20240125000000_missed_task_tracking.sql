-- Track which tasks have been processed for missed penalties on which dates
-- This prevents duplicate penalties when the background job runs multiple times per day
CREATE TABLE IF NOT EXISTS missed_task_penalties (
    task_id TEXT NOT NULL,
    due_date DATE NOT NULL,
    processed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (task_id, due_date),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_missed_task_penalties_date ON missed_task_penalties(due_date);
