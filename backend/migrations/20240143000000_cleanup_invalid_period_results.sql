-- Cleanup invalid task_period_results entries
-- These are entries where period_start is before the task's created_at date,
-- which can happen due to a bug where new tasks were incorrectly marked as
-- "failed" for days before they existed.

DELETE FROM task_period_results
WHERE id IN (
    SELECT tpr.id
    FROM task_period_results tpr
    JOIN tasks t ON tpr.task_id = t.id
    WHERE tpr.period_start < DATE(t.created_at)
);
