## Why

Tasks with Weekdays recurrence (specific days like Sun/Mon/Tue/Wed/Thu) show incorrect dates in the period tracker. Completions on Sunday are incorrectly assigned to the previous week's period, causing the habit tracker to display wrong dates and break streak visualization.

## What Changes

- Fix `get_period_bounds()` to treat Weekdays tasks as daily periods instead of weekly periods
- Each scheduled day in a Weekdays task will track separately (matching user expectation)

## Capabilities

### New Capabilities

(None - this is a bug fix)

### Modified Capabilities

- `task-period-tracking`: Fix period boundary calculation for Weekdays recurrence type to use daily periods instead of weekly

## Impact

- **Backend**: `backend/src/services/scheduler.rs` - Change period type mapping for Weekdays
- **Data**: Existing incorrect period results remain; new completions will be tracked correctly
- **UX**: Users will see accurate completion dates in the period tracker for Weekdays tasks
