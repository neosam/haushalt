## ADDED Requirements

### Requirement: Task Completion Due Date Calculation
The system SHALL correctly calculate completion_due_date for all recurrence types to support period tracking.

#### Scenario: OneTime uses today
- **WHEN** task has OneTime recurrence
- **AND** user completes task today
- **THEN** completion_due_date equals today
- **AND** no scheduled date lookup is performed

#### Scenario: Daily uses today
- **WHEN** task has Daily recurrence
- **AND** user completes task today
- **THEN** completion_due_date equals today
- **AND** period bounds are (today, today)

#### Scenario: Weekly uses next weekly occurrence
- **WHEN** task has Weekly recurrence (e.g., every Monday)
- **AND** user completes task on Tuesday
- **THEN** completion_due_date equals next Monday
- **AND** period bounds are the week containing next Monday

#### Scenario: Weekdays uses next weekday occurrence
- **WHEN** task has Weekdays recurrence (e.g., Mon/Wed/Fri)
- **AND** user completes task on Tuesday
- **THEN** completion_due_date equals next scheduled weekday (Wednesday)
- **AND** period bounds are (Wednesday, Wednesday)

#### Scenario: Custom uses next custom date
- **WHEN** task has Custom recurrence (e.g., [Feb 25, Feb 28])
- **AND** user completes task on Feb 24
- **THEN** completion_due_date equals next custom date (Feb 25)
- **AND** period bounds are (Feb 25, Feb 25)

#### Scenario: Monthly uses next monthly occurrence
- **WHEN** task has Monthly recurrence (e.g., 15th of each month)
- **AND** user completes task on Jan 10
- **THEN** completion_due_date equals Jan 15
- **AND** period bounds are January (Jan 1 - Jan 31)
