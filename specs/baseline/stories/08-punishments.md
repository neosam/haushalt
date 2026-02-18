# Punishments User Stories

## US-PUN-001: Create Punishment

**As a** household Owner or Admin
**I want to** create a punishment
**So that** it can be assigned for missed tasks or bad behavior

### Acceptance Criteria
- Punishment has a name
- Optional description (stored in markdown format, entered via multiline textarea)
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

---

## US-PUN-017: Create Random Choice Punishment

**As a** household Owner or Admin
**I want to** create a punishment that references multiple other punishments
**So that** one is randomly selected when the user needs to fulfill it

### Acceptance Criteria
- Punishment has `punishment_type` set to `random_choice`
- Can link multiple other punishments as options
- At least 2 punishment options must be linked
- Options can include other random choice punishments (nesting allowed)
- Self-reference is allowed (punishment can include itself as an option)
- Shows linked punishments when viewing the punishment
- Punishment type is selected via dropdown (extensible for future types)

---

## US-PUN-018: Link Punishment Option

**As a** household Owner or Admin
**I want to** add a punishment as an option to a random choice punishment
**So that** it can be randomly selected

### Acceptance Criteria
- Target punishment must have `punishment_type = random_choice`
- Option can be any punishment (including self and other random choice punishments)
- Creates link between random choice punishment and option

---

## US-PUN-019: Unlink Punishment Option

**As a** household Owner or Admin
**I want to** remove a punishment option from a random choice punishment
**So that** it's no longer a possible selection

### Acceptance Criteria
- Link is removed
- Random choice punishment must still have at least 2 options after removal

---

## US-PUN-020: Pick Random Punishment

**As a** household member with an assigned random choice punishment
**I want to** click "Pick one" to have the system randomly select a punishment
**So that** I receive a concrete punishment to complete

### Acceptance Criteria
- Only available when user has a random choice punishment assigned
- System randomly selects one of the linked punishment options
- Selected punishment is assigned to the user
- If selected punishment is also a random choice, user must pick again
- Random choice punishment assignment is marked as resolved
- Activity is logged showing which punishment was selected
- Success notification displays which punishment was selected (e.g., "You got: Extra Chores")
