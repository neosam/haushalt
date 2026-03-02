## ADDED Requirements

### Requirement: Filter Tasks by Text
Users SHALL be able to filter task lists using a text search field.

#### Scenario: Filter input field displayed
- **WHEN** user views a page with task lists (Dashboard or Household Overview)
- **THEN** a text input field is displayed above the task list
- **THEN** the field has placeholder text indicating its purpose

#### Scenario: Filter by single word
- **WHEN** user enters a single word in the filter field
- **THEN** only tasks whose titles contain that word (case-insensitive) are shown
- **THEN** tasks not matching the filter are hidden

#### Scenario: Filter by multiple words
- **WHEN** user enters multiple space-separated words
- **THEN** only tasks whose titles contain ALL entered words (case-insensitive) are shown
- **THEN** word order does not matter

#### Scenario: Case-insensitive matching
- **WHEN** user enters text with mixed case (e.g., "Clean")
- **THEN** matches tasks with any casing (e.g., "clean", "CLEAN", "Clean")

#### Scenario: Clear filter
- **WHEN** user clears the text field
- **THEN** all tasks are shown again (respecting other active filters)

#### Scenario: No matches
- **WHEN** filter text matches no task titles
- **THEN** empty task list is shown
- **THEN** appropriate message is displayed

#### Scenario: Works with other filters
- **WHEN** text filter is active alongside other filters (assignment, household)
- **THEN** tasks must match ALL active filter criteria
- **THEN** filter logic is combined with AND operator

#### Scenario: Session persistence
- **WHEN** user applies text filter
- **THEN** filter remains active during session
- **THEN** filter resets on page reload

#### Scenario: Real-time filtering
- **WHEN** user types in the filter field
- **THEN** task list updates immediately as text changes
- **THEN** no submit button is required
