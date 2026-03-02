## ADDED Requirements

### Requirement: Early Completion for Scheduled Recurrences
The system SHALL support completing tasks before their next scheduled occurrence, with the completion counting toward that future occurrence.

#### Scenario: Weekdays early completion on non-scheduled day
- **WHEN** task has Weekdays recurrence (e.g., Mon/Wed/Fri)
- **AND** user completes task on non-scheduled day (e.g., Tuesday)
- **THEN** completion_due_date equals next scheduled day (Wednesday)
- **AND** completion is counted in Wednesday's period, not Tuesday's

#### Scenario: Weekdays completion on scheduled day
- **WHEN** task has Weekdays recurrence (e.g., Mon/Wed/Fri)
- **AND** user completes task on scheduled day (e.g., Monday)
- **AND** Monday has not yet been completed
- **THEN** completion_due_date equals next occurrence of that day (next Monday)
- **AND** completion is counted in next Monday's period

#### Scenario: Custom early completion before next date
- **WHEN** task has Custom recurrence (e.g., [Feb 25, Feb 28, Mar 5])
- **AND** user completes task before next custom date (e.g., Feb 24)
- **THEN** completion_due_date equals next custom date (Feb 25)
- **AND** completion is counted in Feb 25's period

#### Scenario: Custom completion on scheduled date
- **WHEN** task has Custom recurrence (e.g., [Feb 25, Feb 28, Mar 5])
- **AND** user completes task on a custom date (e.g., Feb 25)
- **AND** Feb 25 has not yet been completed
- **THEN** completion_due_date equals next custom date after Feb 25 (Feb 28)
- **AND** completion is counted in Feb 28's period

### Requirement: Multiple Completions Per Scheduled Occurrence
The system SHALL prevent completing the same scheduled occurrence multiple times when allow_exceed_target is false.

#### Scenario: Cannot complete same weekday twice
- **WHEN** task has Weekdays recurrence with allow_exceed_target=false
- **AND** user has already completed next Monday's occurrence
- **AND** user tries to complete again before Monday arrives
- **THEN** system prevents completion (target already met for this period)

#### Scenario: Can complete different weekdays separately
- **WHEN** task has Weekdays recurrence with allow_exceed_target=false
- **AND** user has completed Monday's occurrence
- **AND** user tries to complete Wednesday's occurrence
- **THEN** system allows completion (different period)

#### Scenario: Cannot complete same custom date twice
- **WHEN** task has Custom recurrence with allow_exceed_target=false
- **AND** user has already completed Feb 25 occurrence
- **AND** user tries to complete again (even before Feb 25)
- **THEN** system prevents completion (target already met for Feb 25 period)

#### Scenario: Can complete different custom dates separately
- **WHEN** task has Custom recurrence with allow_exceed_target=false
- **AND** user has completed Feb 25 occurrence
- **AND** user tries to complete Feb 28 occurrence
- **THEN** system allows completion (different period)
