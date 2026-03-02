## Context

The habit tracker is a core feature with complex business logic involving:
- Multiple recurrence types (Daily, Weekly, Monthly, Weekdays, Custom, OneTime)
- Period tracking and finalization
- Streak calculations
- Good vs Bad habit semantics (inverted rewards/penalties)
- Timezone-aware date calculations
- Pause/vacation mode interactions

Currently, the scheduler module has excellent test coverage (32 tests), but service modules have minimal coverage:
- `tasks.rs`: 1 test
- `period_results.rs`: 0 tests
- `task_consequences.rs`: 1 test
- `background_jobs.rs`: 3 tests

This creates risk of regressions when modifying habit tracking logic. The testing infrastructure is basic - each test manually sets up database connections and fixtures.

## Goals / Non-Goals

**Goals:**
- Increase test coverage for habit tracker services from ~5 tests to 70+ tests
- Create reusable test infrastructure (database setup, fixtures, assertions)
- Cover all recurrence types and period tracking edge cases
- Test timezone handling comprehensively
- Test pause/vacation mode interactions
- Test good vs bad habit consequence logic
- Ensure all test scenarios from specs are covered

**Non-Goals:**
- Integration tests hitting HTTP endpoints (focus on service layer)
- Frontend/UI testing (backend only)
- Performance/load testing
- Test coverage metrics tooling (manual verification sufficient)
- Refactoring production code (tests only)

## Decisions

### Decision 1: Test Infrastructure Location
**Choice**: Create `backend/src/test_utils.rs` module for shared test utilities

**Rationale**:
- Rust convention for test utilities
- Can be used across all test modules via `use crate::test_utils::*`
- Keeps test helpers separate from production code
- Easier to maintain than duplicating setup in each test module

**Alternatives considered**:
- Put utilities in each module's test section → leads to duplication
- Separate `test_utils` crate → overkill for current needs

### Decision 2: Database Strategy for Tests
**Choice**: Use in-memory SQLite database per test

**Rationale**:
- Fast test execution (no disk I/O)
- Perfect isolation between tests
- SQLx already supports in-memory databases via `:memory:` connection string
- Migrations can be applied automatically on each test database

**Alternatives considered**:
- Shared test database with cleanup → risk of test pollution
- Temporary file databases → slower, requires cleanup
- Mock database layer → too much mocking overhead, doesn't test SQL queries

### Decision 3: Date/Time Mocking Strategy
**Choice**: Pass `current_date` parameter to functions under test (no global time mocking)

**Rationale**:
- Most scheduler functions already accept date parameters
- Explicit and predictable (no hidden global state)
- Works well with Rust's ownership model
- No need for external crates like `mock_instant`

**Alternatives considered**:
- Global time mocking crate → adds complexity and dependencies
- Modify functions to use trait-based time provider → invasive refactoring

### Decision 4: Test Organization
**Choice**: Keep tests in same file as code using `#[cfg(test)] mod tests`

**Rationale**:
- Rust best practice for unit tests
- Tests close to code being tested
- Can test private functions
- Existing pattern in `scheduler.rs`

**Alternatives considered**:
- Separate `tests/` directory → better for integration tests
- One test file per service → creates too many small files

### Decision 5: Fixture Creation Pattern
**Choice**: Builder-style helpers with sensible defaults

**Example**:
```rust
test_utils::create_task(&pool, household_id)
    .with_title("Exercise")
    .with_recurrence(RecurrenceType::Daily)
    .with_points(10, -5)
    .build()
    .await
```

**Rationale**:
- Readable and expressive
- Reduces boilerplate in tests
- Defaults handle common cases, overrides handle specifics
- Common Rust testing pattern

**Alternatives considered**:
- Struct with all fields → verbose in every test
- Macro-based → harder to understand and debug

### Decision 6: Assertion Helpers
**Choice**: Create domain-specific assertion functions

**Example**:
```rust
assert_completion_exists(&pool, task_id, user_id, CompletionStatus::Approved).await;
assert_streak(&pool, task_id, current: 5, best: 7).await;
assert_period_result(&pool, task_id, date, PeriodStatus::Completed).await;
```

**Rationale**:
- Clearer test intent than raw assertions
- Encapsulates database queries
- Better error messages
- Reduces duplication

**Alternatives considered**:
- Manual database queries in each test → verbose and error-prone
- Generic assertion library → doesn't understand our domain

## Risks / Trade-offs

### [Risk] Tests may become coupled to implementation details
**Mitigation**: Focus assertions on observable outcomes (database state, return values) not internal function calls

### [Risk] In-memory databases may behave differently than production SQLite
**Mitigation**: Use same SQLite version and pragmas in tests. Run manual smoke tests against file database.

### [Risk] Tests may be slow if database setup is expensive
**Mitigation**: Profile test execution. Consider test database pooling if needed. Current migration count (~20) should be fast enough.

### [Risk] Date-based tests may be brittle with hardcoded dates
**Mitigation**: Use relative dates (today + 1, today - 7) where possible. Document why specific dates are used when needed.

### [Risk] Test maintenance burden as code evolves
**Mitigation**: Keep test utilities DRY. Update helper functions once to fix many tests. Delete obsolete tests proactively.

### [Trade-off] Not testing HTTP layer
**Accepted**: Service layer tests provide sufficient coverage for habit logic. HTTP handler tests would be mostly boilerplate with little additional value.

### [Trade-off] Manual test coverage tracking
**Accepted**: Automated coverage tools add complexity. Code review ensures new features have tests (per project requirements).

## Migration Plan

N/A - Tests only, no production code changes or deployment needed.

## Open Questions

None - implementation can proceed with current design decisions.
