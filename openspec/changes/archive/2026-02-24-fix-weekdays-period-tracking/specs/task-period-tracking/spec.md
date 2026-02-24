## MODIFIED Requirements

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
