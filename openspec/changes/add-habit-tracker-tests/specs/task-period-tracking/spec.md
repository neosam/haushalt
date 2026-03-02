## ADDED Requirements

### Requirement: Period Result Test Coverage
The system SHALL have comprehensive tests for period result calculations and edge cases.

#### Scenario: Test period finalization on target reached
- **WHEN** task completion brings count to target
- **THEN** test verifies period result created with 'completed' status
- **THEN** test verifies target_count frozen at finalization

#### Scenario: Test period result update on uncomplete
- **WHEN** task uncomplete drops count below target
- **THEN** test verifies period result is deleted
- **THEN** test verifies period can be re-completed

#### Scenario: Test failed period finalization
- **WHEN** background job processes yesterday's incomplete periods
- **THEN** test verifies period marked as 'failed'
- **THEN** test verifies failed count in statistics

#### Scenario: Test skipped period for paused task
- **WHEN** task is paused during period
- **THEN** test verifies period marked as 'skipped'
- **THEN** test verifies skipped excluded from rate calculation

#### Scenario: Test skipped period for vacation mode
- **WHEN** household in vacation mode during period
- **THEN** test verifies all task periods marked as 'skipped'
- **THEN** test verifies streaks not broken by skipped periods

#### Scenario: Test early completion period assignment
- **WHEN** task completed before scheduled date
- **THEN** test verifies completion_due_date set to next occurrence
- **THEN** test verifies completion counted in future period

#### Scenario: Test multiple completions per period
- **WHEN** allow_exceed_target is true
- **THEN** test verifies multiple completions allowed
- **WHEN** allow_exceed_target is false
- **THEN** test verifies second completion rejected

#### Scenario: Test streak calculation with skipped periods
- **WHEN** streak spans skipped periods
- **THEN** test verifies skipped periods don't break streak
- **THEN** test verifies best streak calculated correctly

#### Scenario: Test completion rate calculation
- **WHEN** calculating completion rate
- **THEN** test verifies skipped periods excluded
- **THEN** test verifies formula: completed / (completed + failed)

#### Scenario: Test timezone handling in period finalization
- **WHEN** households in different timezones
- **THEN** test verifies periods finalized per household timezone
- **THEN** test verifies "yesterday" calculated correctly per timezone
