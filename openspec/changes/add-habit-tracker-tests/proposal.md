## Why

The habit tracker (tasks with recurring schedules) is a core feature of the application, but many service modules have minimal or no test coverage. While the scheduler module has comprehensive tests (32 tests), most task-related services have only 1-2 basic tests. This creates a risk of regressions when modifying task completion logic, period tracking, streak calculations, and habit-specific behavior (good vs bad habits). Comprehensive tests will ensure the habit tracker continues to work correctly as the codebase evolves.

## What Changes

- Add comprehensive unit tests for task service module (task creation, completion, uncompleting, assignment validation)
- Add tests for period results service (period tracking, completion counting, target validation)
- Add tests for task consequences service (rewards/punishments application, good/bad habit logic)
- Add tests for background jobs service (automated punishments, streak updates, auto-archiving)
- Add integration tests for complete habit tracking workflows (daily habits, weekly habits, custom recurrence)
- Add edge case tests for timezone handling in habit tracking
- Add tests for paused tasks and vacation mode interactions

## Capabilities

### New Capabilities
- `testing-infrastructure`: Testing utilities and helpers for habit tracker tests (database setup, fixture creation, assertion helpers)

### Modified Capabilities
- `tasks`: Add test scenarios to existing task requirements to ensure comprehensive coverage of task completion, streaks, and period tracking
- `task-period-tracking`: Add test scenarios for period result calculations and edge cases

## Impact

- **Backend services**: New test modules added to `backend/src/services/` files
- **Test infrastructure**: New test utilities in `backend/src/test_utils.rs` (or similar)
- **CI/CD**: No changes needed - tests run with existing `cargo test --workspace`
- **Coverage**: Test count will increase from ~131 to 200+ tests
- **No breaking changes**: Tests only, no production code changes
