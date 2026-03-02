## ADDED Requirements

### Requirement: Filter Household Tasks by Text
Household task list SHALL support text-based filtering.

#### Scenario: Text filter field on household page
- **WHEN** user views household tasks page
- **THEN** text filter field is displayed above task list
- **THEN** field is placed near existing assignment filter toggle

#### Scenario: Filter household tasks
- **WHEN** user enters text in household tasks filter
- **THEN** only matching tasks from current household are shown
- **THEN** archived tasks section respects filter

#### Scenario: Combine with assignment filter
- **WHEN** "Assigned to Me" filter is active AND text filter is active
- **THEN** only tasks that are assigned to user AND match text are shown
- **THEN** both filter conditions must be satisfied

#### Scenario: Filter includes archived
- **WHEN** text filter is active
- **THEN** filter applies to both active and archived task sections
- **THEN** archived section only shows matching archived tasks

#### Scenario: Filter clears on navigation
- **WHEN** user navigates away from household page
- **THEN** text filter resets
- **THEN** assignment filter also resets (existing behavior)
