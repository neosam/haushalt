# Task Management User Stories

## US-TASK-001: Create Task

**As a** household Owner or Admin (depending on hierarchy)
**I want to** create a new task
**So that** household members can complete it

### Acceptance Criteria
- Task has a title and optional description
- Can set recurrence type:
  - Daily
  - Weekly (with specific day of week)
  - Monthly (with specific day of month)
  - Weekdays (with array of weekday numbers)
  - Custom (with specific dates)
  - OneTime
- Can assign to a specific user
- Can set target count (e.g., "do 3 times per week")
- Can set time period (day, week, month, year)
- Can allow exceeding target count
- Can require review before completion finalizes
- Can set points reward/penalty
- Can set due time
- Can classify as Good or Bad habit
- Can link to a category

---

## US-TASK-002: List Household Tasks

**As a** household member
**I want to** see all tasks in the household
**So that** I know what needs to be done

### Acceptance Criteria
- Returns all tasks with full details
- Shows assigned user if any
- Shows recurrence information
- Shows points value
- Tasks are ordered alphabetically by title

---

## US-TASK-003: View Task Details

**As a** household member
**I want to** view a specific task's details
**So that** I understand what's required

### Acceptance Criteria
- Shows all task properties
- Shows linked category if any
- Shows linked rewards if any
- Shows linked punishments if any

---

## US-TASK-004: Update Task

**As a** household Owner or Admin
**I want to** modify an existing task
**So that** I can adjust requirements

### Acceptance Criteria
- Can modify any task property
- Can clear category assignment
- Changes take effect immediately

---

## US-TASK-005: Delete Task

**As a** household Owner or Admin
**I want to** remove a task
**So that** outdated tasks are cleaned up

### Acceptance Criteria
- Task is permanently removed
- Associated completions are handled appropriately
- Linked rewards/punishments are unlinked

---

## US-TASK-006: Complete Task

**As a** task assignee or Admin (depending on hierarchy)
**I want to** mark a task as completed
**So that** I earn points and rewards

### Acceptance Criteria
- If requires_review: completion status = Pending
- Otherwise: completion status = Approved
- Points are calculated and awarded (if approved)
- Linked rewards are applied (if approved)
- Streak is updated

---

## US-TASK-007: Uncomplete Task

**As the** task completer or Admin
**I want to** remove a task completion
**So that** mistakes can be corrected

### Acceptance Criteria
- Completion record is removed
- Points are reverted
- Rewards are reverted

---

## US-TASK-008: View Due Tasks

**As a** household member
**I want to** see tasks due today or overdue
**So that** I know what needs immediate attention

### Acceptance Criteria
- Returns tasks due today
- Returns overdue tasks
- Sorted by urgency

---

## US-TASK-009: View Tasks with Status

**As a** household member
**I want to** see all tasks with their completion status
**So that** I can track progress

### Acceptance Criteria
- Shows completion count for current period
- Shows current streak
- Shows last completion date
- Shows next due date
- Shows whether user can complete the task
- Shows remaining count until target

---

## US-TASK-010: View Assigned Tasks

**As a** household member
**I want to** see tasks assigned specifically to me
**So that** I can focus on my responsibilities

### Acceptance Criteria
- Returns only tasks where user is the assignee
- Shows completion status for each

---

## US-TASK-011: View Pending Reviews

**As a** household Owner or Admin
**I want to** see task completions awaiting approval
**So that** I can review them

### Acceptance Criteria
- Returns completions with status = Pending
- Shows who completed the task
- Shows when it was completed

---

## US-TASK-012: Approve Task Completion

**As a** household Owner or Admin
**I want to** approve a pending task completion
**So that** the user receives their points and rewards

### Acceptance Criteria
- Completion status changes to Approved
- Points are awarded to the user
- Linked rewards are applied
- Activity is logged

---

## US-TASK-013: Reject Task Completion

**As a** household Owner or Admin
**I want to** reject a pending task completion
**So that** invalid completions are not rewarded

### Acceptance Criteria
- Completion status changes to Rejected
- No points are awarded
- No rewards are applied
- Activity is logged

---

## US-TASK-014: Good Habit Task

**As a** household member
**I want to** have good habit tasks
**So that** I'm rewarded for completing and penalized for missing

### Acceptance Criteria
- Completing a good habit task = reward/points
- Missing a good habit task = penalty/punishment

---

## US-TASK-015: Bad Habit Task

**As a** household member
**I want to** have bad habit tasks
**So that** I'm penalized for completing (indulging) and rewarded for resisting

### Acceptance Criteria
- Completing a bad habit task = penalty/punishment
- Resisting a bad habit task = reward/points
