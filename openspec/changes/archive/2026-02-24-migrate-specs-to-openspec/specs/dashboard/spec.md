## ADDED Requirements

### Requirement: View Dashboard
Authenticated users SHALL see a central dashboard with households and tasks.

#### Scenario: Dashboard layout
- **WHEN** user views dashboard
- **THEN** pending invitations are shown (if any)
- **THEN** create household button is shown
- **THEN** households list is shown (ordered alphabetically)
- **THEN** whitelisted tasks are shown (grouped by household)

#### Scenario: Task display
- **WHEN** tasks are displayed
- **THEN** completion status and progress are shown
- **THEN** tasks are ordered alphabetically by title
- **THEN** archived tasks are excluded

---

### Requirement: Get Dashboard Task IDs
Authenticated users SHALL be able to retrieve their dashboard whitelist.

#### Scenario: Get whitelist
- **WHEN** user requests dashboard task IDs
- **THEN** list of task IDs on whitelist is returned

---

### Requirement: View Dashboard Tasks with Details
Authenticated users SHALL see full task details on dashboard.

#### Scenario: View task details
- **WHEN** user views dashboard
- **THEN** full task details with completion status are shown
- **THEN** household name and ID are shown for each task
- **THEN** whether task can be completed is shown
- **THEN** archived tasks are excluded

---

### Requirement: Add Task to Dashboard
Authenticated users SHALL be able to add tasks to their dashboard.

#### Scenario: Add task
- **WHEN** user adds task to dashboard
- **THEN** task is added to whitelist
- **THEN** task must exist and user must have access

---

### Requirement: Remove Task from Dashboard
Authenticated users SHALL be able to remove tasks from dashboard.

#### Scenario: Remove task
- **WHEN** user removes task from dashboard
- **THEN** task is removed from whitelist
- **THEN** task itself is not affected

---

### Requirement: Check Task on Dashboard
Authenticated users SHALL be able to check if a task is on dashboard.

#### Scenario: Check task
- **WHEN** user checks task dashboard status
- **THEN** true/false is returned

---

### Requirement: Show All Tasks Mode
Authenticated users SHALL be able to see all tasks from all households.

#### Scenario: Enable show all
- **WHEN** user enables "Show all" toggle
- **THEN** all active tasks from all households are shown
- **THEN** tasks are grouped by household
- **THEN** archived tasks are excluded
- **THEN** whitelist is ignored

#### Scenario: Toggle persistence
- **WHEN** user navigates away from dashboard
- **THEN** toggle resets to off

---

### Requirement: Household Name as Link
Household names in task cards SHALL be clickable links.

#### Scenario: Click household name
- **WHEN** user clicks household name in task card
- **THEN** navigates to household page
- **THEN** works in iOS PWA

---

### Requirement: Filter Tasks by Household
Authenticated users SHALL be able to filter dashboard tasks by household.

#### Scenario: Filter households
- **WHEN** user disables household in filter
- **THEN** that household's tasks are hidden

#### Scenario: Filter default
- **WHEN** dashboard loads
- **THEN** all households are enabled

#### Scenario: Filter persistence
- **WHEN** user navigates away
- **THEN** filter resets

---

### Requirement: Responsive Two-Column Layout
Dashboard SHALL display in two columns on desktop.

#### Scenario: Desktop layout
- **WHEN** screen is wide (≥768px)
- **THEN** tasks and households are shown side by side
- **THEN** left column (wider): Show all toggle, filter, tasks
- **THEN** right column: invitations, create button, households

#### Scenario: Mobile layout
- **WHEN** screen is narrow (<768px)
- **THEN** single-column stacked layout

---

### Requirement: Highlight Day Headers
All day headers in task list SHALL be visually highlighted.

#### Scenario: Today header
- **WHEN** "Today" header is displayed
- **THEN** uses primary color (most prominent)

#### Scenario: Other day headers
- **WHEN** other day headers are displayed
- **THEN** use secondary highlighting

---

### Requirement: Toggle Dashboard Whitelist from Dashboard
Users SHALL be able to toggle task whitelist status from dashboard.

#### Scenario: Star toggle
- **WHEN** user clicks star icon on task card
- **THEN** whitelist status is toggled
- **THEN** visual feedback is immediate (optimistic update)

#### Scenario: Star indicator
- **WHEN** task is on whitelist
- **THEN** filled star (★) is shown
- **WHEN** task is not on whitelist
- **THEN** empty star (☆) is shown
