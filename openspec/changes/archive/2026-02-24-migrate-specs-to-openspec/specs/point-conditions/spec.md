## ADDED Requirements

### Requirement: Create Point Condition
Household Owners and Admins SHALL be able to define point condition rules.

#### Scenario: Create TaskComplete condition
- **WHEN** Owner/Admin creates condition with type TaskComplete
- **THEN** points are awarded when tasks are completed

#### Scenario: Create TaskMissed condition
- **WHEN** Owner/Admin creates condition with type TaskMissed
- **THEN** points are deducted when tasks are missed

#### Scenario: Create Streak condition
- **WHEN** Owner/Admin creates condition with type Streak and threshold
- **THEN** points are awarded when streak threshold is reached

#### Scenario: Create StreakBroken condition
- **WHEN** Owner/Admin creates condition with type StreakBroken
- **THEN** points are deducted when streak is broken

#### Scenario: Task-specific condition
- **WHEN** condition is linked to specific task
- **THEN** condition only applies to that task

#### Scenario: Multiplier
- **WHEN** multiplier is set
- **THEN** points are multiplied accordingly

---

### Requirement: List Point Conditions
Household members SHALL be able to see all point rules.

#### Scenario: List conditions
- **WHEN** member requests point conditions
- **THEN** all conditions are returned
- **THEN** condition type, points value, thresholds are shown

---

### Requirement: View Point Condition
Household members SHALL be able to view a specific point condition.

#### Scenario: View condition
- **WHEN** member views condition
- **THEN** all condition properties are shown

---

### Requirement: Update Point Condition
Household Owners and Admins SHALL be able to modify point conditions.

#### Scenario: Update condition
- **WHEN** Owner/Admin updates condition
- **THEN** points value can be modified
- **THEN** thresholds and multipliers can be modified
- **THEN** task association can be changed

---

### Requirement: Delete Point Condition
Household Owners and Admins SHALL be able to remove point conditions.

#### Scenario: Delete condition
- **WHEN** Owner/Admin deletes condition
- **THEN** condition is removed
- **THEN** future events won't use this rule
