## ADDED Requirements

### Requirement: Filter Dashboard Tasks by Text
Dashboard SHALL support text-based task filtering.

#### Scenario: Text filter field on dashboard
- **WHEN** user views dashboard
- **THEN** text filter field is displayed above task list
- **THEN** field is placed below "Show all" toggle and household filter

#### Scenario: Filter dashboard tasks
- **WHEN** user enters text in dashboard filter
- **THEN** only matching tasks from visible households are shown
- **THEN** tasks grouped by household maintain their grouping

#### Scenario: Combine with show all toggle
- **WHEN** "Show all" toggle is enabled AND text filter is active
- **THEN** filter applies to all tasks from all households
- **THEN** matching tasks from all households are shown

#### Scenario: Combine with household filter
- **WHEN** household filter excludes some households AND text filter is active
- **THEN** text filter only applies to tasks from enabled households
- **THEN** disabled households' tasks remain hidden

#### Scenario: Filter clears on navigation
- **WHEN** user navigates away from dashboard
- **THEN** text filter resets
- **THEN** household and show-all filters also reset (existing behavior)
