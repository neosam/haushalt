## Context

The Habit Tracker has fundamental bugs handling Weekdays and Custom recurrence. Previous fixes (e.g., `fix-custom-recurrence-period`) failed because:

1. **Existing tests validate wrong behavior**:
   - Line 694 in scheduler.rs: `assert_eq!(get_next_due_date(&task, monday), Some(monday))`
   - If today is Monday, for early completion it should return the **next** Monday, not today
   - Same problem on line 713 for Custom dates

2. **Missing test coverage**:
   - No tests for early completion scenarios
   - No tests for integration between `get_next_due_date`, `get_period_bounds` and `complete_task`
   - No tests for `allow_exceed_target` with different periods

3. **Symptoms**:
   - Users can't complete tasks in advance
   - Completions are grouped incorrectly
   - `allow_exceed_target=false` blocks legitimate completions

## Goals / Non-Goals

**Goals:**
- **Test-First: Write ~24 new tests BEFORE code fixes**
  - ~15 new unit tests for `get_next_due_date` (scheduler.rs)
  - ~6 new integration tests for `complete_task` (tasks.rs)
  - ~3 new tests for `get_period_bounds` edge cases
  - Fix 2 existing wrong tests
- **Comprehensive Coverage:**
  - All Recurrence Types (OneTime, Daily, Weekly, Monthly, Weekdays, Custom)
  - Early Completion vs. Completion on Scheduled Day
  - Integration: `get_next_due_date` → `complete_task` → `get_period_bounds` → period tracking
  - `allow_exceed_target` edge cases
  - Edge cases: no match, all dates past, etc.
- **Minimal Code Changes:** Only 2 small fixes (loop range, filter condition)
- **Full Verification:** 50+ total tests green, Habit Tracker works

**Non-Goals:**
- Frontend changes (benefits automatically from backend fixes)
- Database migrations (no schema changes)
- Performance optimization (focus: correctness)
- New features (bug fixes only)
- Refactoring (only minimal necessary changes)

## Decisions

### 1. Test-First Approach (Write ~24 Tests BEFORE Fixing Code)

**Decision:** Write all tests with correct expected behavior first, then fix the implementation

**Detailed Workflow:**
1. **Review Phase** (2 tasks):
   - Fix `test_get_next_due_date_weekdays` (line 694): expect next Monday, not today
   - Fix `test_get_next_due_date_custom` (line 713): expect next custom date, not today
   - Run tests → verify they FAIL (current impl returns today)

2. **Write New Unit Tests Phase** (~15 tests):
   - Early completion for Weekdays: `test_get_next_due_date_weekdays_early_completion`, `test_get_next_due_date_weekdays_on_scheduled_day`
   - Early completion for Custom: `test_get_next_due_date_custom_early_completion`, `test_get_next_due_date_custom_on_scheduled_date`
   - Edge cases: `test_weekdays_no_match_in_week`, `test_custom_all_dates_past`, etc.
   - Verification tests for Daily, Weekly, Monthly, OneTime
   - Run tests → verify new tests FAIL (impl not fixed yet)

3. **Write Integration Tests Phase** (~6 tests):
   - `test_complete_weekday_task_early`: verify completion_due_date and period_bounds
   - `test_allow_exceed_target_weekdays`: verify can't complete same day twice
   - `test_allow_exceed_target_different_weekdays`: verify can complete different days
   - Run tests → verify they FAIL (need DB setup + impl fix)

4. **Implementation Phase** (2 small changes):
   - Fix Weekdays: `0..7` → `1..=7`
   - Fix Custom: `>= from_date` → `> from_date`
   - Run tests → verify ALL tests GREEN

5. **Verification Phase**:
   - `cargo test --workspace` → 50+ tests green
   - Manual testing in running app

**Rationale:**
- **Prevents regression:** Tests become source of truth for correct behavior
- **Documents intent:** Each test is an executable requirement
- **Safety net:** Can refactor without fear
- **Forces clarity:** Must understand what's correct BEFORE coding

### 2. Correct Behavior for `get_next_due_date`

**Decision:** For Weekdays and Custom, `get_next_due_date(task, from_date)` must find the **next** scheduled occurrence AFTER `from_date`, not from `from_date` (inclusive)

**Current (Wrong) Behavior:**
```rust
// Weekdays - line 194
for i in 0..7 {  // Starts at from_date
    let check_date = from_date + chrono::Duration::days(i);
    if weekdays.contains(&weekday) {
        return Some(check_date);  // Returns from_date if scheduled
    }
}

// Custom - line 211
dates.iter().filter(|d| **d >= from_date).min()  // >= from_date
```

**Correct Behavior:**
```rust
// Weekdays
for i in 1..=7 {  // Starts at from_date + 1
    let check_date = from_date + chrono::Duration::days(i);
    if weekdays.contains(&weekday) {
        return Some(check_date);  // Returns next scheduled day
    }
}

// Custom
dates.iter().filter(|d| **d > from_date).min()  // > from_date (strict)
```

**Rationale:**
- Enables early completion
- Consistent with Weekly/Monthly behavior (they already have "next occurrence" semantics)
- If today is a scheduled day, completes the task for the next occurrence of that day

**Alternative Considered:**
- "Check if today is scheduled and return today, else next" → Problem: How to decide if today already completed? That belongs in completion logic, not in `get_next_due_date`

### 3. Integration with `complete_task`

**Decision:** `complete_task` (tasks.rs:696-700) calls `get_next_due_date` and uses the result as `completion_due_date`. This is then used for `get_period_bounds`.

**Flow:**
```
User completes task on Tuesday
  ↓
complete_task calls get_next_due_date(task, today=Tuesday)
  ↓
get_next_due_date returns next scheduled day (e.g., Wednesday)
  ↓
completion_due_date = Wednesday
  ↓
get_period_bounds(task, Wednesday) returns (Wednesday, Wednesday) for Weekdays
  ↓
COUNT(*) WHERE due_date >= Wednesday AND due_date <= Wednesday
  ↓
Completion is counted in Wednesday's period
```

**Tests needed:**
- Unit tests for `get_next_due_date` (already exist, need correction)
- Unit tests for `get_period_bounds` (already exist, seem correct)
- **Integration tests** for `complete_task` end-to-end (MISSING!)

### 4. Test Organization

**Decision:** Tests stay in `#[cfg(test)]` modules at the end of scheduler.rs and tasks.rs

**Test Categories:**
1. **Unit Tests - `get_next_due_date`**: One test per recurrence type per scenario
2. **Unit Tests - `get_period_bounds`**: Verify correct period calculation
3. **Integration Tests - `complete_task`**: Database tests (require test DB setup)
4. **Edge Cases**: Custom dates all past, no weekdays in range, etc.

**Rationale:**
- Rust convention: Tests in same file as code
- Use existing test infrastructure
- Clear separation: unit vs. integration

## Risks / Trade-offs

**[Risk] Breaking change for users who rely on current behavior** → Current behavior is buggy (can't early complete). Fix is aligned with user expectations. Mitigate through comprehensive tests.

**[Risk] Tests could be wrong again** → Mitigate through:
1. Write specs BEFORE tests (specs = source of truth)
2. Scenarios in specs = Test cases
3. Review: Each scenario mapped to at least one test

**[Risk] Integration tests need DB setup** → Use existing test DB infrastructure from other services (e.g., tasks.rs already has async tests with SQLite)

**[Trade-off] Test suite grows larger** → Acceptable. Comprehensive tests are exactly what we need. Tests are documentation.

**[Trade-off] More effort upfront** → Pays off through fewer bugs and confidence in future changes
