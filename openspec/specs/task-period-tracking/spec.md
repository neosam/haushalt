## ADDED Requirements

### Requirement: Period Granularity by Recurrence Type
The system SHALL determine period granularity based on recurrence type.

#### Scenario: Daily recurrence uses daily periods
- **WHEN** task has Daily recurrence
- **THEN** each day is tracked as a separate period

#### Scenario: Weekly recurrence uses weekly periods
- **WHEN** task has Weekly recurrence
- **THEN** each week (Mon-Sun) is tracked as a single period

#### Scenario: Weekdays recurrence uses daily periods
- **WHEN** task has Weekdays recurrence (specific days like Sun/Mon/Tue)
- **THEN** each scheduled day is tracked as a separate period
- **AND** period_start equals the completion date (not the week's Monday)

#### Scenario: Monthly recurrence uses monthly periods
- **WHEN** task has Monthly recurrence
- **THEN** each month is tracked as a single period

#### Scenario: Custom/OneTime uses no period grouping
- **WHEN** task has Custom or OneTime recurrence
- **THEN** no period grouping is applied

---

### Requirement: Store Period Results
The system SHALL store the result of each task period for stable statistics.

#### Scenario: Period result stored
- **WHEN** task period ends
- **THEN** result is stored with task_id, period_date, status, completions_count, target_count

#### Scenario: Status values
- **WHEN** period is finalized
- **THEN** status is 'completed', 'failed', or 'skipped'

#### Scenario: Frozen target
- **WHEN** period is finalized
- **THEN** target_count is frozen at finalization time

---

### Requirement: Auto-Finalize Completed Periods
The system SHALL automatically finalize periods when target is reached.

#### Scenario: Target reached
- **WHEN** completion brings completions_count >= target_count
- **THEN** period is finalized as 'completed'

#### Scenario: Upsert behavior
- **WHEN** period result already exists
- **THEN** it is updated (enables failed → completed on late completion)

#### Scenario: Uncomplete drops below target
- **WHEN** uncomplete drops count below target
- **THEN** period result is deleted (allows re-evaluation)

---

### Requirement: Auto-Finalize Failed Periods
The system SHALL automatically mark periods as failed when they end incomplete.

#### Scenario: Background job runs
- **WHEN** background job runs (every minute)
- **THEN** checks each household's timezone for "yesterday"

#### Scenario: Finalize failed
- **WHEN** yesterday's period is unfinalized with completions < target
- **THEN** period is finalized as 'failed'

#### Scenario: Finalize completed
- **WHEN** completions >= target but no result exists
- **THEN** period is finalized as 'completed'

#### Scenario: Skip already finalized
- **WHEN** period is already finalized
- **THEN** no update is made

---

### Requirement: Skip Periods for Paused Tasks
The system SHALL mark paused task periods as skipped.

#### Scenario: Paused task period
- **WHEN** period is finalized for paused task
- **THEN** status is 'skipped'

#### Scenario: Vacation mode period
- **WHEN** period is finalized during vacation mode
- **THEN** status is 'skipped'

#### Scenario: Skipped excluded from rate
- **WHEN** completion rate is calculated
- **THEN** skipped periods are excluded

#### Scenario: Skipped don't break streak
- **WHEN** streak is calculated
- **THEN** skipped periods don't break streak

---

### Requirement: Calculate Statistics from Period Results
The system SHALL calculate statistics from stored period results.

#### Scenario: Completion rate formula
- **WHEN** completion rate is calculated
- **THEN** rate = completed / (completed + failed) × 100%

#### Scenario: Exclude skipped
- **WHEN** rate is calculated
- **THEN** skipped periods are excluded

#### Scenario: Current streak
- **WHEN** streak is calculated
- **THEN** counts consecutive 'completed' results (skipped continue)

#### Scenario: Best streak
- **WHEN** best streak is calculated
- **THEN** finds longest consecutive 'completed' run

---

### Requirement: Display Period Results
Users SHALL see period results in task views.

#### Scenario: Habit tracker display
- **WHEN** task is displayed
- **THEN** last 15 periods shown as inline icons

#### Scenario: Visual indicators
- **WHEN** periods are displayed
- **THEN** ✓ = completed, ✗ = failed, - = skipped

#### Scenario: In-progress indicator
- **WHEN** today has no entry yet
- **THEN** ○ (in progress) is shown

#### Scenario: Bad habit colors
- **WHEN** task is bad habit
- **THEN** colors are inverted (completed = red, failed = green)

#### Scenario: Hover tooltip
- **WHEN** user hovers over period icon
- **THEN** date is shown
