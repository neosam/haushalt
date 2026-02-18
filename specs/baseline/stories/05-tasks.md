# Task Management User Stories

## US-TASK-001: Create Task

**As a** household Owner or Admin (depending on hierarchy)
**I want to** create a new task
**So that** household members can complete it

### Acceptance Criteria
- Task has a title and optional description (stored in markdown format, entered via multiline textarea)
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
**I want to** see all active tasks in the household
**So that** I know what needs to be done

### Acceptance Criteria
- Returns all active (non-archived) tasks with full details
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
- If task has an assigned user, only that user can complete the task
- If task has no assigned user, any household member can complete it
- If requires_review: completion status = Pending
- Otherwise: completion status = Approved
- Points are calculated and awarded (if approved)
- Linked rewards are applied (if approved)
- Streak is updated
- The +/- buttons are hidden for users who cannot complete the task

---

## US-TASK-007: Uncomplete Task

**As the** task completer or Admin
**I want to** remove a task completion
**So that** mistakes can be corrected

### Acceptance Criteria
- If task has an assigned user, only that user can uncomplete (remove their own completions)
- If task has no assigned user, any household member can uncomplete their own completions
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

---

## US-TASK-016: Archive Task

**As a** household Owner or Admin
**I want to** archive a task
**So that** it is hidden from active task lists but preserved for history

### Acceptance Criteria
- Task can be archived via context menu in the UI
- Archived tasks are excluded from the main task list (household tasks view)
- Archived tasks are excluded from the main page (dashboard)
- Archived tasks are excluded from due tasks
- Archived tasks are excluded from household overview
- Task completion history is preserved
- Archived tasks are viewable in a collapsible "Archived Tasks" section at the bottom of the tasks page
- Archived tasks can be unarchived via context menu to restore them to active status

### Design Decisions
- **Archive UI location**: Collapsible section at bottom of tasks page (decided over toggle or tabs)

---

## US-TASK-017: Quick Task Creation

**As a** user with task creation permission in one or more households
**I want to** quickly create a task from anywhere in the app
**So that** I can capture tasks without navigating to a specific household's task page

### Acceptance Criteria
- A floating action button (+) appears in the lower-right corner on all authenticated pages
- Button uses circular Material Design FAB style
- On click:
  - Fetches households where user has task creation permission (based on `HierarchyType.can_manage(role)`)
  - If user has permission in exactly one household: opens the task creation modal directly for that household
  - If user has permission in multiple households: shows a household selection modal first, then opens task creation modal
  - If user has no permission in any household: shows appropriate message
- Uses the existing task creation modal (US-TASK-001)
- Recurrence type defaults to "OneTime" (instead of the usual "Daily" default) for quick task creation
- After successful task creation, the modal closes and the new task appears in the appropriate household

### Design Decisions
- **FAB placement**: Fixed position, lower-right corner (follows Material Design conventions)
- **Single household optimization**: Skips selection step when only one option exists
- **Global visibility**: Available on all authenticated pages for quick access

---

## US-TASK-018: Pause Task

**As a** household Owner or Admin
**I want to** pause individual tasks
**So that** no automated punishments are given while the task is temporarily inactive

### Acceptance Criteria
- Task can be paused via context menu or task edit modal
- Paused tasks are visually distinguished (e.g., muted/grayed appearance, pause indicator)
- While paused:
  - Task does not appear in due tasks
  - No automated punishments are triggered for missed completions
  - No streak penalties are applied
  - Task remains visible in the task list (not hidden like archived)
- Paused tasks can still be manually completed if desired
- Task can be unpaused to resume normal behavior
- Pause/unpause actions are logged in activity

### Design Decisions
- **Pause vs Archive**: Paused tasks remain visible and can be completed; archived tasks are hidden entirely
- **Punishment handling**: Only automated punishments are suppressed; manual actions still work

---

## US-TASK-019: Household Vacation Mode

**As a** household Owner
**I want to** put the entire household in vacation mode
**So that** all tasks are paused during the vacation period without manual intervention

### Acceptance Criteria
- Vacation mode can be enabled/disabled in household settings
- Optional: Set vacation start and end dates for automatic activation/deactivation
- When vacation mode is active:
  - All tasks in the household are effectively paused
  - No automated punishments are triggered for any task
  - No streak penalties are applied
  - Tasks do not appear in due tasks
  - A banner or indicator shows the household is in vacation mode
- When vacation mode ends:
  - Tasks resume normal behavior
  - Streaks continue from where they left off (no reset)
- Vacation mode status is visible to all household members
- Enabling/disabling vacation mode is logged in activity

### Design Decisions
- **Scope**: Affects all tasks in the household uniformly
- **Streak handling**: Streaks are preserved, not reset, when vacation ends
- **Override**: Individual task pause (US-TASK-018) and vacation mode are independent; a task paused before vacation remains paused after vacation ends

---

## US-TASK-020: Task Detail View

**As a** household member
**I want to** view a detailed, well-formatted modal for a task
**So that** I can see all task information, statistics, and linked items in an easy-to-read format

### Acceptance Criteria

#### Task Information Display
- Shows task title prominently in modal header
- Renders description in nicely formatted markdown (not raw markdown text)
- Shows task type (Good/Bad habit) with appropriate visual indicator
- Shows recurrence information in human-readable format:
  - Daily → "Every day"
  - Weekly(3) → "Every Wednesday"
  - Monthly(15) → "On the 15th of each month"
  - Weekdays([1,3,5]) → "Every Monday, Wednesday, Friday"
  - Custom → "On specific dates: ..."
  - OneTime → "One-time task"
- Shows due time if set
- Shows target count and time period (e.g., "3 times per week")
- Shows whether exceeding target is allowed
- Shows whether review is required before approval
- Shows assigned user if any
- Shows linked category if any

#### Completion Statistics
- **Completion rates** shown for three time periods:
  - **This week**: Percentage for current week with breakdown (e.g., "80% (4 of 5 periods)")
  - **This month**: Percentage for current month with breakdown
  - **All time**: Percentage since task creation with breakdown
  - Calculated as: (periods where target was met / total applicable periods) × 100%
- **Current streak**: Number of consecutive successful periods
- **Best streak**: Historical record of longest successful streak
- **Total completions**: Cumulative count of all individual completions
- **Last completed**: Date and time of most recent completion
- **Next due**: Date when task is next due

#### Points Information
- **Points on completion**: Points awarded when task is completed (for good habits) or deducted (for bad habits)
- **Points on miss**: Points deducted when task is missed (for good habits) or awarded for resisting (for bad habits)
- Clear visual distinction between positive and negative point values (e.g., +10 in green, -5 in red)

#### Linked Rewards and Punishments
- **Linked rewards**: List of all rewards triggered when task is completed
  - Shows reward name with amount (e.g., "Movie Night x2")
- **Linked punishments**: List of all punishments triggered when task is missed
  - Shows punishment name with amount

#### Navigation and Actions
- "Edit" button for users with edit permission (opens edit modal)
- Close button (X) to dismiss the modal
- Quick complete/uncomplete action available in the modal

### Design Decisions
- **UI pattern**: Modal overlay (consistent with notes/rewards patterns in the app)
- **Navigation trigger**: Click on task title opens the detail modal. Available from:
  - Main dashboard (tasks across all households)
  - Household overview page (task cards)
  - Tasks page (task list within a household)
- **Read-only focus**: This view emphasizes readability; editing is done through a separate edit modal
- **Statistics periods**: Completion rates shown for week, month, and all-time to provide both recent and historical context
- **Mobile-friendly**: Layout adapts for smaller screens with stacked sections
