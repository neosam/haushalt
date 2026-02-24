## ADDED Requirements

### Requirement: Create Task
Household Owners and Admins (depending on hierarchy) SHALL be able to create tasks.

#### Scenario: Create basic task
- **WHEN** Owner/Admin creates task with title
- **THEN** task is created in household
- **THEN** task is visible to all members

#### Scenario: Task with description
- **WHEN** task is created with description
- **THEN** description is stored in markdown format

#### Scenario: Set recurrence type
- **WHEN** task is created with recurrence type (Daily/Weekly/Monthly/Weekdays/Custom/OneTime)
- **THEN** task appears on scheduled days

#### Scenario: Assign to user
- **WHEN** task is assigned to specific user
- **THEN** only that user can complete the task

#### Scenario: Set target count
- **WHEN** target count is set (e.g., 3 times per week)
- **THEN** task tracks completions against target

#### Scenario: Allow exceeding target
- **WHEN** allow_exceed_target is true
- **THEN** user can complete beyond target count

#### Scenario: Require review
- **WHEN** requires_review is true
- **THEN** completions need approval before points are awarded

#### Scenario: Set points
- **WHEN** points_reward and points_penalty are set
- **THEN** points are awarded/deducted accordingly

#### Scenario: Set due time
- **WHEN** due_time is set
- **THEN** task shows due time in UI

#### Scenario: Set habit type
- **WHEN** habit_type is set to Good or Bad
- **THEN** completion/miss behavior reflects habit type

#### Scenario: Link to category
- **WHEN** category is selected
- **THEN** task is associated with that category

---

### Requirement: List Household Tasks
Household members SHALL be able to see all active tasks.

#### Scenario: View active tasks
- **WHEN** member requests task list
- **THEN** all active (non-archived) tasks are returned
- **THEN** tasks are ordered alphabetically by title

#### Scenario: Task details shown
- **WHEN** tasks are listed
- **THEN** assigned user is shown (if any)
- **THEN** recurrence information is shown
- **THEN** points value is shown

#### Scenario: Assignment badge
- **WHEN** task is assigned to current user
- **THEN** "Assigned to you" badge is displayed on task card

#### Scenario: Bad habit badge
- **WHEN** task is a bad habit
- **THEN** "Bad" badge is displayed on task card

---

### Requirement: View Task Details
Household members SHALL be able to view detailed task information.

#### Scenario: View task details
- **WHEN** member views task details
- **THEN** all task properties are shown
- **THEN** linked category is shown (if any)
- **THEN** linked rewards are shown (if any)
- **THEN** linked punishments are shown (if any)

---

### Requirement: Update Task
Household Owners and Admins SHALL be able to modify tasks.

#### Scenario: Modify task properties
- **WHEN** Owner/Admin updates task
- **THEN** any property can be modified
- **THEN** changes take effect immediately

#### Scenario: Clear category
- **WHEN** Owner/Admin clears category assignment
- **THEN** task is no longer associated with category

---

### Requirement: Delete Task
Household Owners and Admins SHALL be able to remove tasks.

#### Scenario: Delete task
- **WHEN** Owner/Admin deletes task
- **THEN** task is permanently removed
- **THEN** linked rewards/punishments are unlinked

---

### Requirement: Complete Task
Assignees or authorized members SHALL be able to mark tasks as completed.

#### Scenario: Complete assigned task
- **WHEN** assigned user completes their task
- **THEN** completion is recorded

#### Scenario: Complete unassigned task
- **WHEN** any member completes unassigned task
- **THEN** completion is recorded

#### Scenario: Cannot complete others' assigned task
- **WHEN** user attempts to complete task assigned to someone else
- **THEN** completion is rejected

#### Scenario: Completion with review
- **WHEN** requires_review is true
- **THEN** completion status is Pending

#### Scenario: Completion without review
- **WHEN** requires_review is false
- **THEN** completion status is Approved
- **THEN** points are awarded
- **THEN** linked rewards are applied

#### Scenario: Streak updated
- **WHEN** task is completed
- **THEN** streak is updated

#### Scenario: Hidden buttons for unauthorized
- **WHEN** user cannot complete the task
- **THEN** +/- buttons are hidden

---

### Requirement: Uncomplete Task
Task completers SHALL be able to remove their completions.

#### Scenario: Uncomplete own completion
- **WHEN** user removes their completion
- **THEN** completion record is removed
- **THEN** points are reverted
- **THEN** rewards are reverted

#### Scenario: Assigned task uncomplete
- **WHEN** assigned user uncompletes
- **THEN** only their own completions are removed

---

### Requirement: View Due Tasks
Household members SHALL be able to see tasks due today or overdue.

#### Scenario: View due tasks
- **WHEN** member requests due tasks
- **THEN** tasks due today are returned
- **THEN** overdue tasks are returned
- **THEN** sorted by urgency

---

### Requirement: View Tasks with Status
Household members SHALL be able to see tasks with completion status.

#### Scenario: Task status information
- **WHEN** member views tasks with status
- **THEN** completion count for current period is shown
- **THEN** current streak is shown
- **THEN** last completion date is shown
- **THEN** next due date is shown
- **THEN** remaining count until target is shown
- **THEN** whether user can complete is shown

---

### Requirement: View Assigned Tasks
Household members SHALL be able to filter tasks assigned to them.

#### Scenario: View assigned tasks
- **WHEN** member requests their assigned tasks
- **THEN** only tasks where user is assignee are returned
- **THEN** completion status is shown for each

---

### Requirement: View Pending Reviews
Household Owners and Admins SHALL be able to see completions awaiting approval.

#### Scenario: View pending reviews
- **WHEN** Owner/Admin requests pending reviews
- **THEN** completions with Pending status are returned
- **THEN** shows who completed the task
- **THEN** shows when it was completed

---

### Requirement: Approve Task Completion
Household Owners and Admins SHALL be able to approve pending completions.

#### Scenario: Approve completion
- **WHEN** Owner/Admin approves completion
- **THEN** status changes to Approved
- **THEN** points are awarded
- **THEN** linked rewards are applied
- **THEN** activity is logged

---

### Requirement: Reject Task Completion
Household Owners and Admins SHALL be able to reject pending completions.

#### Scenario: Reject completion
- **WHEN** Owner/Admin rejects completion
- **THEN** status changes to Rejected
- **THEN** no points are awarded
- **THEN** no rewards are applied
- **THEN** activity is logged

---

### Requirement: Good Habit Task
Good habit tasks SHALL reward completion and penalize misses.

#### Scenario: Complete good habit
- **WHEN** good habit task is completed
- **THEN** reward/points are awarded

#### Scenario: Miss good habit
- **WHEN** good habit task is missed
- **THEN** penalty/punishment is applied

---

### Requirement: Bad Habit Task
Bad habit tasks SHALL penalize completion and reward resistance.

#### Scenario: Complete bad habit (indulge)
- **WHEN** bad habit task is completed
- **THEN** penalty/punishment is applied

#### Scenario: Resist bad habit
- **WHEN** bad habit task is not completed (resisted)
- **THEN** reward/points are awarded

---

### Requirement: Archive Task
Household Owners and Admins SHALL be able to archive tasks.

#### Scenario: Archive task
- **WHEN** Owner/Admin archives task
- **THEN** task is hidden from active task lists
- **THEN** task is excluded from due tasks
- **THEN** task is excluded from dashboard
- **THEN** completion history is preserved

#### Scenario: View archived tasks
- **WHEN** user views tasks page
- **THEN** archived tasks appear in collapsible section at bottom

#### Scenario: Unarchive task
- **WHEN** Owner/Admin unarchives task
- **THEN** task returns to active status

---

### Requirement: Quick Task Creation
Users with task creation permission SHALL be able to create tasks from anywhere.

#### Scenario: FAB visibility
- **WHEN** user is authenticated
- **THEN** floating action button (+) appears in lower-right corner

#### Scenario: Single household
- **WHEN** user has permission in exactly one household
- **THEN** clicking FAB opens task creation modal directly

#### Scenario: Multiple households
- **WHEN** user has permission in multiple households
- **THEN** clicking FAB shows household selection first
- **THEN** households are ordered alphabetically

#### Scenario: No permission
- **WHEN** user has no permission in any household
- **THEN** appropriate message is shown

#### Scenario: OneTime default
- **WHEN** task is created via quick creation
- **THEN** recurrence defaults to OneTime

---

### Requirement: Pause Task
Household Owners and Admins SHALL be able to pause individual tasks.

#### Scenario: Pause task
- **WHEN** Owner/Admin pauses task
- **THEN** task is visually distinguished
- **THEN** task does not appear in due tasks
- **THEN** no automated punishments are triggered
- **THEN** no streak penalties are applied

#### Scenario: Manual completion while paused
- **WHEN** task is paused
- **THEN** task can still be manually completed

#### Scenario: Unpause task
- **WHEN** Owner/Admin unpauses task
- **THEN** normal behavior resumes

#### Scenario: Activity logged
- **WHEN** task is paused or unpaused
- **THEN** action is logged in activity

---

### Requirement: Household Vacation Mode
Household Owners SHALL be able to put entire household in vacation mode.

#### Scenario: Enable vacation mode
- **WHEN** Owner enables vacation mode
- **THEN** all tasks are effectively paused
- **THEN** no automated punishments are triggered
- **THEN** no streak penalties are applied
- **THEN** tasks do not appear in due tasks

#### Scenario: Vacation with dates
- **WHEN** vacation start and end dates are set
- **THEN** vacation activates/deactivates automatically

#### Scenario: Vacation ends
- **WHEN** vacation mode ends
- **THEN** tasks resume normal behavior
- **THEN** streaks continue (not reset)

#### Scenario: Banner displayed
- **WHEN** vacation mode is active
- **THEN** banner shows status to all members

---

### Requirement: Task Detail View
Household members SHALL be able to view detailed task modal.

#### Scenario: View task information
- **WHEN** user clicks task title
- **THEN** detail modal opens
- **THEN** title is shown prominently
- **THEN** description is rendered as markdown
- **THEN** habit type indicator is shown
- **THEN** recurrence is shown in human-readable format

#### Scenario: Completion statistics
- **WHEN** detail modal is open
- **THEN** completion rates for week/month/all-time are shown
- **THEN** current streak is shown
- **THEN** best streak is shown
- **THEN** total completions are shown
- **THEN** last completed date is shown
- **THEN** next due date is shown

#### Scenario: Points information
- **WHEN** detail modal is open
- **THEN** points on completion are shown
- **THEN** points on miss are shown
- **THEN** positive values shown in green, negative in red

#### Scenario: Linked items
- **WHEN** detail modal is open
- **THEN** linked rewards with amounts are shown
- **THEN** linked punishments with amounts are shown

#### Scenario: Edit action
- **WHEN** user has edit permission
- **THEN** Edit button is available

---

### Requirement: Auto-Archive Obsolete Tasks
The system SHALL automatically archive completed one-time and custom tasks.

#### Scenario: Auto-archive one-time
- **WHEN** one-time task is completed
- **THEN** after grace period, task is auto-archived

#### Scenario: Auto-archive custom
- **WHEN** custom task is completed AND last date has passed
- **THEN** after grace period, task is auto-archived

#### Scenario: Never auto-archive uncompleted
- **WHEN** task is not completed
- **THEN** task is NEVER auto-archived

#### Scenario: Configurable grace period
- **WHEN** auto_archive_days is set in household settings
- **THEN** grace period uses that value

#### Scenario: Activity logging
- **WHEN** task is auto-archived
- **THEN** TaskAutoArchived activity is logged

---

### Requirement: Suggest Task
Members without task creation permission SHALL be able to suggest tasks.

#### Scenario: Suggest task
- **WHEN** member without permission suggests task
- **THEN** task is created with suggestion status 'suggested'

#### Scenario: FAB for suggestions
- **WHEN** user can suggest but not create
- **THEN** FAB shows "Suggest" badge for that household

#### Scenario: Household setting
- **WHEN** allow_task_suggestions is disabled
- **THEN** members cannot suggest tasks

#### Scenario: Review suggestions
- **WHEN** Owner/Admin views suggestions
- **THEN** pending suggestions are shown
- **THEN** can approve or deny

#### Scenario: Approve suggestion
- **WHEN** suggestion is approved
- **THEN** task becomes active

#### Scenario: Deny suggestion
- **WHEN** suggestion is denied
- **THEN** task remains in history with denied status

---

### Requirement: Task Card Context Menu
Household members SHALL be able to access quick actions via context menu.

#### Scenario: Context menu button
- **WHEN** task card is displayed
- **THEN** "⋮" button appears for quick actions

#### Scenario: Edit action
- **WHEN** user has edit permission
- **THEN** Edit option opens task edit modal

#### Scenario: Set Date action
- **WHEN** task has no schedule
- **THEN** Set Date option is available
- **THEN** selecting date converts to Custom recurrence

#### Scenario: Pause/Unpause action
- **WHEN** user has edit permission
- **THEN** Pause/Unpause toggle is available

#### Scenario: Solo Mode restrictions
- **WHEN** Solo Mode is active
- **THEN** Edit and Pause actions are hidden
- **THEN** Set Date remains available

---

### Requirement: Filter Tasks by Assignment
Household members SHALL be able to filter task list by assignment.

#### Scenario: Filter toggle
- **WHEN** user views task list
- **THEN** assignment filter toggle is available

#### Scenario: Filter active
- **WHEN** "Assigned to Me" filter is active
- **THEN** only tasks assigned to current user are shown
- **THEN** unassigned tasks are hidden

#### Scenario: Session persistence
- **WHEN** filter is set
- **THEN** filter persists during session
- **THEN** filter resets on page reload

---

### Requirement: Bulk Edit Multiple Tasks
Household Owners and Admins SHALL be able to edit multiple tasks at once.

#### Scenario: Enter multi-select mode
- **WHEN** user activates multi-select
- **THEN** checkboxes appear on task cards
- **THEN** selection toolbar appears

#### Scenario: Select tasks
- **WHEN** user selects tasks
- **THEN** selection count is shown
- **THEN** Edit Selected button becomes available

#### Scenario: Bulk edit modal
- **WHEN** user opens bulk edit
- **THEN** modal shows "Edit X Tasks"
- **THEN** fields have "Apply" checkboxes
- **THEN** only checked fields are updated

#### Scenario: Editable fields
- **WHEN** bulk edit is active
- **THEN** category, assigned user, recurrence, target count can be edited
- **THEN** points, habit type, paused status can be edited
- **THEN** title and description are NOT editable

#### Scenario: Save behavior
- **WHEN** changes are applied
- **THEN** selected tasks are updated sequentially
- **THEN** progress is shown

#### Scenario: Partial failure
- **WHEN** some tasks fail to update
- **THEN** error summary is shown
- **THEN** retry option is available
