# Task Period Tracking

> **Status:** Implemented (Backend + Frontend)
> **Created:** 2026-02-19
> **Implemented:** 2026-02-19

## Problem

Currently, task statistics are calculated on-the-fly by counting completions and comparing against `target_count`. This has several issues:

| Issue | Description |
|-------|-------------|
| **Unstable history** | Changing `target_count` retroactively changes past statistics |
| **No failure records** | Only successful completions are stored, not missed days |
| **Performance** | Statistics must be recalculated on every request |
| **No snapshot** | No record of what the target was when a period ended |

## Solution

Introduce a `task_period_results` table that explicitly records the outcome of each period (day/week/month) when it ends.

---

## US-PERIOD-001: Store Period Results

**As a** system
**I want to** store the result of each task period
**so that** statistics are stable and accurate

### Acceptance Criteria

- New table `task_period_results` stores period outcomes
- Each record contains: task_id, period_date, status, completions_count, target_count
- Status can be: `completed`, `failed`, or `skipped`
- Target count is frozen at finalization time
- Unique constraint on (task_id, period_start)

### Database Schema

```sql
CREATE TABLE task_period_results (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    status TEXT NOT NULL,                -- 'completed' | 'failed' | 'skipped'
    completions_count INTEGER NOT NULL,
    target_count INTEGER NOT NULL,
    finalized_at DATETIME NOT NULL,
    finalized_by TEXT,                   -- 'system' | 'user' | 'migration'
    notes TEXT,

    UNIQUE(task_id, period_start)
);

CREATE INDEX idx_period_results_task ON task_period_results(task_id);
CREATE INDEX idx_period_results_status ON task_period_results(status);
```

---

## US-PERIOD-002: Auto-Finalize Completed Periods

**As a** system
**I want to** automatically finalize periods when the target is reached
**so that** successful completions are immediately recorded

### Acceptance Criteria

- When a completion brings `completions_count >= target_count`, create a `completed` result
- If period result already exists, update it (upsert behavior) - enables `failed` → `completed` on late completion
- Store the `target_count` at the time of finalization
- `finalized_by` is set to 'system'
- When uncomplete drops below target, delete the period result (allows re-evaluation)

---

## US-PERIOD-003: Auto-Finalize Failed Periods

**As a** system
**I want to** automatically mark periods as failed when they end
**so that** missed days are recorded for statistics

### Acceptance Criteria

- Background job runs every minute (to handle different timezones)
- Uses household timezone to determine "yesterday"
- For each task, check if yesterday's period is unfinalized
- If `completions < target`, create a `failed` result
- If `completions >= target` but no result exists, create `completed` result
- Skip tasks that are paused or in vacation mode (mark as `skipped`)
- Already finalized periods are skipped (no redundant updates)

---

## US-PERIOD-004: Skip Periods for Paused Tasks

**As a** system
**I want to** mark periods as skipped when a task is paused
**so that** paused periods don't count as failures

### Acceptance Criteria

- When finalizing a period for a paused task, status is `skipped`
- Vacation mode periods are also `skipped`
- Skipped periods do NOT count towards completion rate
- Skipped periods do NOT break streaks

---

## US-PERIOD-005: Calculate Statistics from Period Results

**As a** system
**I want to** calculate statistics from period results
**so that** statistics are fast and stable

### Acceptance Criteria

- Completion rate = completed / (completed + failed) × 100%
- Skipped periods are excluded from the calculation
- Current streak counts consecutive `completed` results (skipped don't break)
- Best streak is the longest consecutive `completed` run

### Formulas

```
completion_rate = completed / (completed + failed) × 100%
                = completed / (total - skipped) × 100%

streak: completed +1, skipped continue, failed break
```

---

## US-PERIOD-006: Backfill Historical Data

> **Status:** Denied
> **Reason:** Retroactive calculations would be inaccurate - historical `target_count` values are not available, and past periods cannot be reliably reconstructed.

**As a** system
**I want to** backfill period results for existing tasks
**so that** historical statistics are available

### Acceptance Criteria

- ~~Migration script processes all existing tasks~~
- ~~For each past period, count completions and determine status~~
- ~~Use current `target_count` (historical values not available)~~
- ~~Mark as `finalized_by = 'migration'`~~

**Decision:** Statistics start fresh from implementation date. Existing tasks show 0/0 until new periods are recorded going forward.

---

## US-PERIOD-007: Display Period Results

> **Status:** Implemented
> **Implemented:** 2026-02-19

**As a** user
**I want to** see period results in the task detail view
**so that** I can see my completion history

### Acceptance Criteria

- Habit tracker style: show last 15 periods as inline icons
- Visual indicators: ✓ completed, ✗ failed, - skipped
- Icons displayed horizontally in a row (oldest left, newest right)
- Hover/tooltip shows the date of each period
- Today's period shown as "in progress" (○ or similar)
- Display in **both** list views (Dashboard, Household Overview, Task List) and detail view
- Statistics show completed/failed/skipped counts

### UI Mockup

```
Letzte Perioden: ✓ ✓ ✗ ✓ ✓ ✓ - ✓ ✓ ✗ ✓ ✓ ✓ ✓ ○
                                              ↑
                                          [19.02.2026]
                                          (on hover)
```

---

## Technical Notes

### Finalization Timing

| Event | Action |
|-------|--------|
| Completion reaches target | Finalize as `completed` |
| Midnight passes | Finalize yesterday as `failed` (if incomplete) |
| Task paused | Future periods finalized as `skipped` |
| Late completion | Update `failed` → `completed` |
| Uncomplete drops below target | Delete period result (can be re-evaluated) |

### Statistics Query

```sql
SELECT
    COUNT(*) FILTER (WHERE status = 'completed') as completed,
    COUNT(*) FILTER (WHERE status = 'failed') as failed,
    COUNT(*) FILTER (WHERE status = 'skipped') as skipped
FROM task_period_results
WHERE task_id = ?
  AND period_start >= ?
  AND period_start <= ?
```

---

## Implementation Notes

### Backend Implementation (Completed)

| Component | File | Description |
|-----------|------|-------------|
| Migration | `backend/migrations/20240138000000_task_period_results.sql` | Creates the `task_period_results` table |
| Shared Types | `shared/src/types.rs` | `PeriodStatus` enum, `TaskPeriodResult` struct, updated `TaskStatistics` |
| Model | `backend/src/models/task_period_result.rs` | `TaskPeriodResultRow` database model |
| Service | `backend/src/services/period_results.rs` | Core period result operations |
| Integration | `backend/src/services/tasks.rs` | Auto-finalize on completion in `complete_task()`, delete on uncomplete in `uncomplete_task()` |
| Background Job | `backend/src/services/background_jobs.rs` | `process_period_finalization()` for failed/skipped periods |
| Statistics | `backend/src/services/tasks.rs` | Updated `calculate_task_statistics()` to include skipped counts |

### Statistics Calculation

Statistics are now calculated **exclusively** from `task_period_results`:
- `periods_completed` = COUNT where status = 'completed'
- `periods_total` = completed + failed (excludes skipped)
- `completion_rate` = completed / total × 100% (or None if total = 0)

Tasks without period results will show 0/0 and no completion rate.

### Timezone Handling

The background job respects each household's timezone setting:
- "Yesterday" is calculated per-household based on their timezone
- Job runs every minute to catch midnight in all timezones
- Already finalized periods are skipped (efficient, no redundant updates)

### Frontend Implementation (Completed)

| Component | File | Description |
|-----------|------|-------------|
| Shared Types | `shared/src/types.rs` | `PeriodDisplay` struct, `recent_periods` added to `TaskWithStatus` and `TaskWithDetails` |
| Period Tracker | `frontend/src/components/period_tracker.rs` | `PeriodTracker` and `PeriodTrackerCompact` components |
| Task Card | `frontend/src/components/task_card.rs` | Displays compact period tracker in list views |
| Task Detail | `frontend/src/components/task_detail_modal.rs` | Displays period tracker in detail view |
| Styles | `frontend/styles.css` | Period tracker CSS styles |
| Translations | `frontend/src/translations/*.json` | "Recent Periods" / "Letzte Perioden" |

### Pending

| Item | Description | Impact |
|------|-------------|--------|
| **Streak calculation** | Update to use period results instead of completions | Streaks still use old completion-based calculation |

### Denied

| Item | Reason |
|------|--------|
| **US-PERIOD-006 (Backfill)** | Retroactive calculations would be inaccurate (no historical target_count) |
