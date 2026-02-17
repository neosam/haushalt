# Punishments User Stories

## US-PUN-001: Create Punishment

**As a** household Owner or Admin
**I want to** create a punishment
**So that** it can be assigned for missed tasks or bad behavior

### Acceptance Criteria
- Punishment has a name
- Optional description
- Requires confirmation flag (must admin approve completion)

---

## US-PUN-002: List Household Punishments

**As a** household member
**I want to** see all available punishments
**So that** I know what consequences exist

### Acceptance Criteria
- Returns all punishments in the household
- Shows name, description
- Shows if requires confirmation

---

## US-PUN-003: View Punishment Details

**As a** household member
**I want to** view a specific punishment
**So that** I understand what's required

### Acceptance Criteria
- Shows all punishment properties

---

## US-PUN-004: Update Punishment

**As a** household Owner or Admin
**I want to** modify a punishment
**So that** I can adjust its properties

### Acceptance Criteria
- Can modify name and description
- Can change confirmation requirement

---

## US-PUN-005: Delete Punishment

**As a** household Owner or Admin
**I want to** remove a punishment
**So that** outdated punishments are cleaned up

### Acceptance Criteria
- Punishment is removed
- Task links to this punishment are removed

---

## US-PUN-006: Assign Punishment to User

**As a** household Owner or Admin
**I want to** assign a punishment to a user
**So that** they must complete it

### Acceptance Criteria
- UserPunishment entry is created
- Used for automatic task penalty assignment
- Activity is logged

---

## US-PUN-007: Unassign Punishment from User

**As a** household Owner or Admin
**I want to** remove a punishment from a user
**So that** I can correct mistakes

### Acceptance Criteria
- UserPunishment entry is removed
- User no longer has that punishment assigned

---

## US-PUN-008: View My Punishments

**As a** household member
**I want to** see punishments assigned to me
**So that** I know what I need to complete

### Acceptance Criteria
- Shows all my UserPunishment entries
- Shows total amount
- Shows completed amount
- Shows pending completion amount

---

## US-PUN-009: View All User Punishments

**As a** household Owner or Admin
**I want to** see all members' punishments
**So that** I can manage the punishment system

### Acceptance Criteria
- Shows all UserPunishment entries in household
- Shows which user has each punishment

---

## US-PUN-010: Complete Punishment

**As a** household member
**I want to** mark a punishment as completed
**So that** I fulfill my obligation

### Acceptance Criteria
- If requires_confirmation: status = Pending
- Otherwise: status = Approved immediately
- Completed amount is incremented

---

## US-PUN-011: View Pending Completions

**As a** household Owner or Admin
**I want to** see punishments awaiting confirmation
**So that** I can approve or reject them

### Acceptance Criteria
- Returns all completions with status = Pending
- Shows who is completing
- Shows which punishment

---

## US-PUN-012: Approve Completion

**As a** household Owner or Admin
**I want to** approve a pending completion
**So that** the user's obligation is fulfilled

### Acceptance Criteria
- Completion status changes to Approved
- Completed amount is finalized
- Activity is logged

---

## US-PUN-013: Reject Completion

**As a** household Owner or Admin
**I want to** reject a pending completion
**So that** incomplete punishments aren't cleared

### Acceptance Criteria
- Completion status changes to Rejected
- Pending amount is removed
- Activity is logged

---

## US-PUN-014: Link Punishment to Task

**As a** household Owner or Admin
**I want to** attach a punishment to a task
**So that** missing the task automatically assigns the punishment

### Acceptance Criteria
- Punishment is linked to task
- Amount can be specified (how many times to apply)
- Punishment is assigned when task is missed

---

## US-PUN-015: Unlink Punishment from Task

**As a** household Owner or Admin
**I want to** remove a punishment from a task
**So that** it's no longer assigned automatically

### Acceptance Criteria
- Punishment link is removed
- Future missed tasks don't assign this punishment

---

## US-PUN-016: View Task Linked Punishments

**As a** household member
**I want to** see punishments attached to a task
**So that** I know the consequences of missing it

### Acceptance Criteria
- Shows all punishments linked to the task
- Shows the amount for each
