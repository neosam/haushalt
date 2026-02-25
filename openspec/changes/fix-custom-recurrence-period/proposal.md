## Why

Custom recurrence tasks (tasks with specific dates like [Feb 25, Feb 28, Mar 5]) incorrectly use an "all-time" period (1970-2100), causing all completions to be lumped together. This prevents tracking each custom date independently and blocks users from completing tasks for different custom dates when `allow_exceed_target=false`.

## What Changes

- Change Custom recurrence from `TimePeriod::None` (all-time) to `TimePeriod::Day` in `get_period_bounds`
- Each custom date will be tracked as its own independent period, similar to how Weekdays recurrence works after the recent fix (commit c772ad8)
- Early completions will correctly count toward the next scheduled custom date

## Capabilities

### New Capabilities

None - this is a bug fix to existing functionality.

### Modified Capabilities

- `task-period-tracking`: Custom recurrence tasks will track each custom date as a separate period instead of using a single all-time period

## Impact

- **Backend**: `backend/src/services/scheduler.rs` - `get_period_bounds` function
- **Database**: No schema changes; existing Custom task period results may have incorrect `period_start/end` dates (1970/2100), but new completions will create correct per-date records
- **API**: No changes to API contracts
- **Frontend**: No changes needed; will automatically display correct period tracking
