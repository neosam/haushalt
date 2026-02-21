-- Backfill task_period_results for completed OneTime tasks
-- This ensures OneTime tasks appear in weekly/monthly statistics

-- Insert period_results for OneTime tasks that:
-- 1. Have recurrence_type = 'onetime'
-- 2. Have completions meeting or exceeding target_count
-- 3. Don't already have a period_result

INSERT INTO task_period_results (
    id,
    task_id,
    period_start,
    period_end,
    status,
    completions_count,
    target_count,
    finalized_at,
    finalized_by,
    notes
)
SELECT
    lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab',abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))) as id,
    t.id as task_id,
    date(tc_last.completed_at) as period_start,
    date(tc_last.completed_at) as period_end,
    'completed' as status,
    tc_count.completion_count as completions_count,
    t.target_count as target_count,
    tc_last.completed_at as finalized_at,
    'migration' as finalized_by,
    'Backfilled by migration for statistics' as notes
FROM tasks t
-- Get completion count per task
INNER JOIN (
    SELECT task_id, COUNT(*) as completion_count
    FROM task_completions
    GROUP BY task_id
) tc_count ON tc_count.task_id = t.id
-- Get the last completion date per task
INNER JOIN (
    SELECT task_id, MAX(completed_at) as completed_at
    FROM task_completions
    GROUP BY task_id
) tc_last ON tc_last.task_id = t.id
WHERE t.recurrence_type = 'onetime'
  AND tc_count.completion_count >= t.target_count
  AND NOT EXISTS (
    SELECT 1 FROM task_period_results tpr
    WHERE tpr.task_id = t.id
  );
