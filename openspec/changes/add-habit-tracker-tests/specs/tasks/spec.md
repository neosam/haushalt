## ADDED Requirements

### Requirement: Task Completion Test Coverage
The system SHALL have comprehensive tests for task completion logic and validation.

#### Scenario: Test complete assigned task
- **WHEN** assigned user completes task
- **THEN** test verifies completion record created
- **THEN** test verifies completion status reflects requires_review setting

#### Scenario: Test complete unassigned task
- **WHEN** any user completes unassigned task
- **THEN** test verifies completion allowed
- **THEN** test verifies completion attributed to user

#### Scenario: Test reject completion of others' task
- **WHEN** user tries to complete task assigned to someone else
- **THEN** test verifies completion rejected with NotAssigned error
- **THEN** test verifies no completion record created

#### Scenario: Test task completion with review
- **WHEN** task requires review and is completed
- **THEN** test verifies completion status is Pending
- **THEN** test verifies no points awarded yet

#### Scenario: Test task completion without review
- **WHEN** task does not require review and is completed
- **THEN** test verifies completion status is Approved
- **THEN** test verifies points awarded immediately

#### Scenario: Test uncomplete own completion
- **WHEN** user uncompletes their completion
- **THEN** test verifies completion record removed
- **THEN** test verifies points reverted

#### Scenario: Test cannot uncomplete others' completion
- **WHEN** user tries to uncomplete someone else's completion
- **THEN** test verifies operation rejected
- **THEN** test verifies completion record unchanged

### Requirement: Task Streak Test Coverage
The system SHALL have comprehensive tests for streak calculation logic.

#### Scenario: Test streak increments on consecutive completion
- **WHEN** task completed on consecutive periods
- **THEN** test verifies current_streak increments
- **THEN** test verifies best_streak updated if higher

#### Scenario: Test streak resets on missed period
- **WHEN** task period ends as failed
- **THEN** test verifies current_streak resets to 0
- **THEN** test verifies best_streak preserved

#### Scenario: Test streak preserved during pause
- **WHEN** task paused for some periods
- **THEN** test verifies streak not broken by skipped periods
- **THEN** test verifies streak continues after unpause

#### Scenario: Test streak preserved during vacation
- **WHEN** household in vacation mode
- **THEN** test verifies all task streaks preserved
- **THEN** test verifies streaks continue after vacation ends

### Requirement: Task Consequences Test Coverage
The system SHALL have comprehensive tests for good/bad habit reward and punishment logic.

#### Scenario: Test good habit completion awards rewards
- **WHEN** good habit task completed
- **THEN** test verifies positive points awarded
- **THEN** test verifies linked rewards applied

#### Scenario: Test good habit miss applies penalties
- **WHEN** good habit task period fails
- **THEN** test verifies negative points deducted
- **THEN** test verifies linked punishments applied

#### Scenario: Test bad habit completion applies penalties
- **WHEN** bad habit task completed
- **THEN** test verifies negative points deducted
- **THEN** test verifies linked punishments applied

#### Scenario: Test bad habit resistance awards rewards
- **WHEN** bad habit task period ends without completion
- **THEN** test verifies positive points awarded
- **THEN** test verifies linked rewards applied

#### Scenario: Test paused task skips consequences
- **WHEN** task is paused
- **THEN** test verifies no penalties applied on period end
- **THEN** test verifies manual completion still works

### Requirement: Task Creation Test Coverage
The system SHALL have comprehensive tests for task creation and validation.

#### Scenario: Test create task with valid fields
- **WHEN** task created with all required fields
- **THEN** test verifies task saved to database
- **THEN** test verifies all fields match input

#### Scenario: Test create task with defaults
- **WHEN** task created with minimal fields
- **THEN** test verifies default values applied correctly
- **THEN** test verifies target_count defaults to 1

#### Scenario: Test create daily task
- **WHEN** task created with Daily recurrence
- **THEN** test verifies recurrence_type stored
- **THEN** test verifies task due every day

#### Scenario: Test create weekly task
- **WHEN** task created with Weekly recurrence and weekday
- **THEN** test verifies recurrence_value stored
- **THEN** test verifies task due on correct weekday

#### Scenario: Test create custom recurrence task
- **WHEN** task created with Custom recurrence and dates list
- **THEN** test verifies custom dates stored
- **THEN** test verifies task due only on custom dates

#### Scenario: Test create task with category
- **WHEN** task created with category_id
- **THEN** test verifies category linked
- **THEN** test verifies category_name populated

### Requirement: Task Auto-Archive Test Coverage
The system SHALL have comprehensive tests for auto-archive logic.

#### Scenario: Test auto-archive one-time task after completion
- **WHEN** one-time task completed and grace period passed
- **THEN** test verifies task auto-archived
- **THEN** test verifies TaskAutoArchived activity logged

#### Scenario: Test auto-archive custom task after last date
- **WHEN** custom task last date passed and grace period expired
- **THEN** test verifies task auto-archived
- **THEN** test verifies activity logged

#### Scenario: Test never auto-archive incomplete tasks
- **WHEN** one-time task not completed
- **THEN** test verifies task never auto-archived
- **THEN** test verifies task remains active indefinitely

#### Scenario: Test configurable grace period
- **WHEN** household sets auto_archive_days
- **THEN** test verifies grace period honored
- **THEN** test verifies archive happens at correct time
