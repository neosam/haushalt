# Dashboard User Stories

## US-DASH-001: View Dashboard

**As an** authenticated user
**I want to** see a central dashboard
**So that** I can quickly access my households and tasks

### Acceptance Criteria
- Dashboard layout order (top to bottom):
  1. Pending invitations section (if any)
  2. Create household button
  3. Households section (list of user's households)
  4. Tasks section (whitelisted tasks)
- Households are ordered alphabetically by name
- Shows whitelisted tasks from all households
- Groups tasks by household
- Shows completion status and progress
- Tasks are ordered alphabetically by title within each category
- Archived tasks are excluded even if on whitelist

---

## US-DASH-002: Get Dashboard Task IDs

**As an** authenticated user
**I want to** retrieve my dashboard task whitelist
**So that** I know which tasks are on my dashboard

### Acceptance Criteria
- Returns list of task IDs on user's dashboard

---

## US-DASH-003: View Dashboard Tasks with Details

**As an** authenticated user
**I want to** see full task details on my dashboard
**So that** I can understand and complete tasks

### Acceptance Criteria
- Returns full task details with completion status
- Shows household name and ID for each task
- Shows whether task can be completed
- Excludes archived tasks

---

## US-DASH-004: Add Task to Dashboard

**As an** authenticated user
**I want to** add a task to my dashboard
**So that** I can track it centrally

### Acceptance Criteria
- Task is added to user's whitelist
- Task must exist and user must have access

---

## US-DASH-005: Remove Task from Dashboard

**As an** authenticated user
**I want to** remove a task from my dashboard
**So that** I can declutter my view

### Acceptance Criteria
- Task is removed from user's whitelist
- Task itself is not affected

---

## US-DASH-006: Check Task on Dashboard

**As an** authenticated user
**I want to** check if a task is on my dashboard
**So that** I can toggle its presence

### Acceptance Criteria
- Returns true/false for whether task is on dashboard

---

## US-DASH-007: Show All Tasks Mode

**As an** authenticated user
**I want to** optionally see all tasks from all my households on the dashboard
**So that** I have a complete overview without manually adding each task

### Acceptance Criteria
- Toggle/switch on the dashboard labeled "Show all" (off by default)
- When enabled, dashboard shows all active tasks from all households the user is a member of
- Tasks are still grouped by household
- Archived tasks are excluded
- Overrides the whitelist behavior (whitelist is ignored when this mode is enabled)
- Toggle state resets to off when navigating away from the dashboard

---

## US-DASH-008: Household Name as Link

> **Status:** Implemented (Bug: iOS PWA click not working)
> **Implemented:** 2026-02-19

**As an** authenticated user
**I want to** click on the household name in a task card
**So that** I can quickly navigate to that household's page

### Acceptance Criteria
- Household name in task card meta line is a clickable link
- Link navigates to `/households/{household_id}`
- Link is styled distinctly (primary color, underline on hover)
- Works in dashboard task list view
- **Must work on iOS PWA** (use same event handler fix as task clicking)

---

## US-DASH-009: Filter Tasks by Household

> **Status:** Implemented
> **Implemented:** 2026-02-20

**As an** authenticated user
**I want to** filter dashboard tasks by household
**So that** I can focus on tasks from specific households

### Acceptance Criteria
- Filter UI displayed above the task list on the dashboard
- Shows list of all user's households as toggleable filters
- All households are enabled by default
- User can disable/enable individual households
- Disabled households' tasks are hidden from the task list
- Filter state resets when navigating away from the dashboard
- Works in combination with "Show all" mode (US-DASH-007)

---

## US-DASH-010: Responsive Two-Column Layout

> **Status:** Implemented
> **Implemented:** 2026-02-20

**As an** authenticated user on a desktop device
**I want to** see households and tasks in separate columns
**So that** I can view more information at once on larger screens

### Acceptance Criteria
- On desktop/wide screens: display tasks and households side by side in two columns
  - Left column (wider): Show all toggle, Household filter, Tasks list
  - Right column: Pending invitations, Create household button, Households list
- On mobile/narrow screens: single-column stacked layout with households on top, tasks below
- Use CSS media queries for responsive breakpoint (768px)
- Column ratio 2:1 (tasks column is wider)

---

## US-DASH-011: Highlight All Day Headers in Task List

> **Status:** Implemented
> **Implemented:** 2026-02-21

**As an** authenticated user
**I want to** see all day headers (Today, Tomorrow, weekdays, dates) visually highlighted
**So that** I can easily see where each day's tasks begin and end

### Acceptance Criteria
- All day group headers should be visually distinct and highlighted
- "Today" header uses primary color (most prominent)
- Other day headers (Tomorrow, weekdays, future dates) use secondary highlighting
- Clear visual separation between different days' task groups
- Applies to both Dashboard and Household overview task lists

---

## US-DASH-012: Toggle Dashboard Whitelist from Dashboard

> **Status:** Implemented
> **Implemented:** 2026-02-21

**As an** authenticated user viewing the dashboard
**I want to** toggle whether a task is on my dashboard whitelist directly from the dashboard
**So that** I can manage my whitelisted tasks without navigating to each household

### Acceptance Criteria
- Star icon (★/☆) appears on each task card in the dashboard task list
- Filled star (★) indicates task is on whitelist, empty star (☆) indicates it would only appear in "Show all" mode
- Clicking the star toggles the task's whitelist status
- Visual feedback is immediate (optimistic UI update)
- Works in both normal mode and "Show all" mode
- Consistent with the star toggle behavior on household overview page
