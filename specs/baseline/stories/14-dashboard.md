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

> **Status:** Implemented
> **Implemented:** 2026-02-19

**As an** authenticated user
**I want to** click on the household name in a task card
**So that** I can quickly navigate to that household's page

### Acceptance Criteria
- Household name in task card meta line is a clickable link
- Link navigates to `/households/{household_id}`
- Link is styled distinctly (primary color, underline on hover)
- Works in dashboard task list view
