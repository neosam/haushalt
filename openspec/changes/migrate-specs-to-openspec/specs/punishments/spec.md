## ADDED Requirements

### Requirement: Create Punishment
Household Owners and Admins SHALL be able to create punishments.

#### Scenario: Create punishment
- **WHEN** Owner/Admin creates punishment with name
- **THEN** punishment is created

#### Scenario: Optional description
- **WHEN** description is provided
- **THEN** description is stored in markdown format

#### Scenario: Requires confirmation
- **WHEN** requires_confirmation is set
- **THEN** completions need admin approval

---

### Requirement: List Household Punishments
Household members SHALL be able to see all punishments.

#### Scenario: List punishments
- **WHEN** member requests punishments
- **THEN** all punishments are returned
- **THEN** name, description, confirmation flag are shown

---

### Requirement: View Punishment Details
Household members SHALL be able to view a specific punishment.

#### Scenario: View punishment
- **WHEN** member views punishment
- **THEN** all punishment properties are shown

---

### Requirement: Update Punishment
Household Owners and Admins SHALL be able to modify punishments.

#### Scenario: Update punishment
- **WHEN** Owner/Admin updates punishment
- **THEN** name and description can be modified
- **THEN** confirmation requirement can be changed

---

### Requirement: Delete Punishment
Household Owners and Admins SHALL be able to remove punishments.

#### Scenario: Delete punishment
- **WHEN** Owner/Admin deletes punishment
- **THEN** punishment is removed
- **THEN** task links are removed

---

### Requirement: Assign Punishment to User
Household Owners and Admins SHALL be able to assign punishments.

#### Scenario: Assign punishment
- **WHEN** Owner/Admin assigns punishment to user
- **THEN** UserPunishment entry is created
- **THEN** activity is logged

---

### Requirement: Unassign Punishment from User
Household Owners and Admins SHALL be able to remove punishments from users.

#### Scenario: Unassign punishment
- **WHEN** Owner/Admin removes punishment from user
- **THEN** UserPunishment entry is removed

---

### Requirement: View My Punishments
Household members SHALL be able to see their assigned punishments.

#### Scenario: View my punishments
- **WHEN** member requests their punishments
- **THEN** all UserPunishment entries are returned
- **THEN** total amount, completed amount, pending amount are shown

---

### Requirement: View All User Punishments
Household Owners and Admins SHALL be able to see all members' punishments.

#### Scenario: View all user punishments
- **WHEN** Owner/Admin requests all user punishments
- **THEN** all UserPunishment entries in household are returned
- **THEN** shows which user has each punishment

---

### Requirement: Complete Punishment
Household members SHALL be able to mark punishments as completed.

#### Scenario: Complete with confirmation
- **WHEN** member completes punishment that requires confirmation
- **THEN** status is set to Pending

#### Scenario: Complete without confirmation
- **WHEN** member completes punishment that doesn't require confirmation
- **THEN** status is set to Approved immediately
- **THEN** completed amount is incremented

---

### Requirement: View Pending Completions
Household Owners and Admins SHALL be able to see punishments awaiting confirmation.

#### Scenario: View pending
- **WHEN** Owner/Admin requests pending completions
- **THEN** all completions with Pending status are returned
- **THEN** shows who and which punishment

---

### Requirement: Approve Completion
Household Owners and Admins SHALL be able to approve pending completions.

#### Scenario: Approve completion
- **WHEN** Owner/Admin approves completion
- **THEN** status changes to Approved
- **THEN** completed amount is finalized
- **THEN** activity is logged

---

### Requirement: Reject Completion
Household Owners and Admins SHALL be able to reject pending completions.

#### Scenario: Reject completion
- **WHEN** Owner/Admin rejects completion
- **THEN** status changes to Rejected
- **THEN** pending amount is removed
- **THEN** activity is logged

---

### Requirement: Link Punishment to Task
Household Owners and Admins SHALL be able to attach punishments to tasks.

#### Scenario: Link punishment
- **WHEN** Owner/Admin links punishment to task with amount
- **THEN** missing task assigns the punishment

---

### Requirement: Unlink Punishment from Task
Household Owners and Admins SHALL be able to remove punishments from tasks.

#### Scenario: Unlink punishment
- **WHEN** Owner/Admin removes punishment from task
- **THEN** future misses don't assign this punishment

---

### Requirement: View Task Linked Punishments
Household members SHALL be able to see punishments attached to tasks.

#### Scenario: View task punishments
- **WHEN** member views task punishments
- **THEN** all linked punishments are shown
- **THEN** amount for each is shown

---

### Requirement: Create Random Choice Punishment
Owners and Admins SHALL be able to create punishments with random selection.

#### Scenario: Create random choice punishment
- **WHEN** punishment is created with punishment_type = random_choice
- **THEN** punishment can have multiple options linked

#### Scenario: Minimum options
- **WHEN** random choice punishment is created
- **THEN** at least 2 options must be linked

#### Scenario: Nested random choice
- **WHEN** random choice punishment options are set
- **THEN** options can include other random choice punishments
- **THEN** self-reference is allowed

---

### Requirement: Link Punishment Option
Owners and Admins SHALL be able to add options to random choice punishments.

#### Scenario: Link option
- **WHEN** Owner/Admin adds punishment as option to random choice punishment
- **THEN** link is created

---

### Requirement: Unlink Punishment Option
Owners and Admins SHALL be able to remove options from random choice punishments.

#### Scenario: Unlink option
- **WHEN** Owner/Admin removes option from random choice punishment
- **THEN** link is removed
- **THEN** random choice must still have at least 2 options

---

### Requirement: Pick Random Punishment
Members with random choice punishments SHALL be able to randomly select one.

#### Scenario: Pick random
- **WHEN** member clicks "Pick one" on random choice punishment
- **THEN** system randomly selects one option
- **THEN** selected punishment is assigned to user
- **THEN** random choice assignment is marked resolved
- **THEN** activity is logged

#### Scenario: Nested random choice selected
- **WHEN** selected punishment is also random choice
- **THEN** user must pick again
