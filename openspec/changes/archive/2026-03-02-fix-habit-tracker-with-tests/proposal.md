## Why

The Habit Tracker has fundamental bugs with Weekdays and Custom recurrence. Previous fixes failed because **existing tests validate wrong behavior**. Line 694 in scheduler.rs tests `get_next_due_date(&task, monday)` and expects `monday` back - this prevents early completion. We need a test-first approach: Write correct tests specifying desired behavior first, then fix the implementation.

## What Changes

**Phase 1: Write comprehensive test suite (Test-First)**

- **Fix 2 existing wrong tests** in `backend/src/services/scheduler.rs` (lines 694, 713)
- **Write ~15 new unit tests** for `get_next_due_date`:
  - Early completion scenarios for Weekdays (2 tests)
  - Early completion scenarios for Custom (2 tests)
  - Edge cases: no weekdays in range, all custom dates past, etc. (3 tests)
  - Verification tests for other recurrence types (Daily, Weekly, Monthly, OneTime) (4 tests)
- **Write ~6 new integration tests** for `complete_task`:
  - Early completion end-to-end with DB (2 tests)
  - `allow_exceed_target=false` for different periods (2 tests)
  - Period bounds verification (2 tests)
- **Write ~3 new tests** for `get_period_bounds` edge cases

**Phase 2: Fix implementation based on failing tests**

- Fix `get_next_due_date` for Weekdays (change loop range)
- Fix `get_next_due_date` for Custom (change filter condition)

**Phase 3: Verification**

- Run full test suite (~50+ tests total)
- Verify Habit Tracker works correctly in running app

## Capabilities

### New Capabilities

None - this is fixing existing functionality with proper tests.

### Modified Capabilities

- `task-period-tracking`: Weekdays and Custom recurrence must support early completion, period tracking must work correctly per scheduled date
- `tasks`: Task completion must work correctly with period bounds and due dates for all recurrence types

## Impact

- **Backend**: `backend/src/services/scheduler.rs` - Tests and implementation of `get_next_due_date` and `get_period_bounds`
- **Backend**: `backend/src/services/tasks.rs` - Tests for integration with `complete_task`
- **Tests**: Existing tests will be corrected, new tests added
- **Database**: No schema changes
- **API**: No API changes
- **Frontend**: No changes (benefits automatically from backend fixes)
