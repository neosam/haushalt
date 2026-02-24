## ADDED Requirements

### Requirement: Configure Week Start Day
Household Owners SHALL be able to configure which day starts the week.

#### Scenario: Set week start day
- **WHEN** Owner sets week_start_day (0-6, 0=Monday)
- **THEN** statistics align with configured day

#### Scenario: Default value
- **WHEN** week_start_day is not set
- **THEN** defaults to 0 (Monday)

---

### Requirement: Calculate Weekly Statistics
The system SHALL calculate weekly statistics for each household member.

#### Scenario: Weekly calculation
- **WHEN** week ends (on configured week start day)
- **THEN** statistics are calculated for previous week

#### Scenario: Statistics content
- **WHEN** statistics are calculated
- **THEN** includes total expected completions
- **THEN** includes total actual completions
- **THEN** includes per-task breakdown
- **THEN** includes overall completion rate

#### Scenario: Hierarchy filtering
- **WHEN** statistics are calculated
- **THEN** only includes tasks where member is "on hierarchy"

#### Scenario: Archived tasks
- **WHEN** statistics are calculated
- **THEN** archived tasks are excluded from new calculations

---

### Requirement: Calculate Monthly Statistics
The system SHALL calculate monthly statistics for each household member.

#### Scenario: Monthly calculation
- **WHEN** month ends (on 1st of next month)
- **THEN** statistics are calculated for previous month

#### Scenario: Calendar month
- **WHEN** month is defined
- **THEN** uses 1st to last day of calendar month

---

### Requirement: Background Job for Statistics
The system SHALL automatically calculate statistics on schedule.

#### Scenario: Daily check
- **WHEN** background job runs daily
- **THEN** checks each household's timezone and week_start_day

#### Scenario: Weekly trigger
- **WHEN** it's the week start day
- **THEN** previous week's statistics are calculated

#### Scenario: Monthly trigger
- **WHEN** it's the 1st of the month
- **THEN** previous month's statistics are calculated

#### Scenario: Idempotent
- **WHEN** job re-runs
- **THEN** no duplicate records are created

---

### Requirement: View Weekly Statistics Page
Household members SHALL be able to view weekly statistics.

#### Scenario: Statistics tab
- **WHEN** member navigates to Statistics tab
- **THEN** weekly statistics are displayed

#### Scenario: Member overview
- **WHEN** statistics are displayed
- **THEN** shows each member's username
- **THEN** shows overall completion rate with progress bar
- **THEN** shows total completed / expected

#### Scenario: Expandable detail
- **WHEN** member expands their row
- **THEN** per-task breakdown is shown

#### Scenario: Visibility rules
- **WHEN** member views statistics
- **THEN** can only see own detailed breakdown unless Owner/Admin

---

### Requirement: View Monthly Statistics Page
Household members SHALL be able to view monthly statistics.

#### Scenario: Toggle view
- **WHEN** user toggles to monthly view
- **THEN** monthly statistics are displayed

#### Scenario: Month display
- **WHEN** monthly view is shown
- **THEN** month name and year are in header

---

### Requirement: Browse Historical Statistics
Household members SHALL be able to browse past statistics.

#### Scenario: Navigate periods
- **WHEN** user navigates to previous/next period
- **THEN** statistics for that period are shown

#### Scenario: No data message
- **WHEN** period has no statistics
- **THEN** "No statistics available" is shown

#### Scenario: Cannot navigate future
- **WHEN** user attempts to navigate to future
- **THEN** navigation is blocked

---

### Requirement: Statistics API Endpoints
The system SHALL provide API endpoints for statistics.

#### Scenario: Get weekly statistics
- **WHEN** GET /api/households/{id}/statistics/weekly is called
- **THEN** weekly statistics for all visible members are returned

#### Scenario: Get monthly statistics
- **WHEN** GET /api/households/{id}/statistics/monthly is called
- **THEN** monthly statistics for all visible members are returned

#### Scenario: User detail endpoint
- **WHEN** GET /api/households/{id}/statistics/weekly/{user_id} is called
- **THEN** detailed statistics for that user are returned
- **THEN** only accessible by user themselves or Owner/Admin

---

### Requirement: Statistics Calculation Logic
The system SHALL correctly calculate statistics from period results.

#### Scenario: Assigned task responsibility
- **WHEN** task has assigned_user_id
- **THEN** only that user is responsible

#### Scenario: Shared task - Equals hierarchy
- **WHEN** task is shared in Equals hierarchy
- **THEN** all members share responsibility (divide by count)

#### Scenario: Shared task - Hierarchy mode
- **WHEN** task is shared in Hierarchy mode
- **THEN** only Members are responsible

#### Scenario: Expected vs completed
- **WHEN** counts are calculated
- **THEN** expected = period results where responsible
- **THEN** completed = 'completed' results where responsible
- **THEN** skipped are excluded

#### Scenario: OneTime task handling
- **WHEN** OneTime task reaches target
- **THEN** creates period_result with completion date
- **THEN** appears in statistics for that period

#### Scenario: Bad habit handling
- **WHEN** bad habit statistics are calculated
- **THEN** success = NOT completing (resisting)
- **THEN** failure = completing (giving in)

---

### Requirement: Statistics Data Retention
The system SHALL define data retention policy.

#### Scenario: Weekly retention
- **WHEN** weekly statistics are stored
- **THEN** retained for 2 years

#### Scenario: Monthly retention
- **WHEN** monthly statistics are stored
- **THEN** retained indefinitely

#### Scenario: Household deletion
- **WHEN** household is deleted
- **THEN** all statistics are deleted
