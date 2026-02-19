-- Task Period Results: Stores the outcome of each task period for stable statistics
CREATE TABLE IF NOT EXISTS task_period_results (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('completed', 'failed', 'skipped')),
    completions_count INTEGER NOT NULL,
    target_count INTEGER NOT NULL,
    finalized_at DATETIME NOT NULL,
    finalized_by TEXT NOT NULL DEFAULT 'system',
    notes TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_period_results_task_date
    ON task_period_results(task_id, period_start);
CREATE INDEX IF NOT EXISTS idx_period_results_status
    ON task_period_results(status);
CREATE INDEX IF NOT EXISTS idx_period_results_task
    ON task_period_results(task_id);
