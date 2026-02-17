# Dashboard User Stories

## US-DASH-001: View Dashboard

**As an** authenticated user
**I want to** see a central dashboard
**So that** I can quickly access my tasks across households

### Acceptance Criteria
- Shows whitelisted tasks from all households
- Groups tasks by household
- Shows completion status and progress

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
