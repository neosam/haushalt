# Point Conditions User Stories

## US-PC-001: Create Point Condition

**As a** household Owner or Admin
**I want to** define a point condition rule
**So that** points are awarded/deducted automatically for events

### Acceptance Criteria
- Condition type must be specified:
  - TaskComplete: Points when task is completed
  - TaskMissed: Points when task is missed
  - Streak: Points for maintaining a streak
  - StreakBroken: Points when streak is broken
- Points value (positive or negative)
- Optional streak threshold (for streak-based conditions)
- Optional multiplier
- Optional task-specific condition (applies only to specific task)

---

## US-PC-002: List Point Conditions

**As a** household member
**I want to** see all point rules in the household
**So that** I understand how points are earned

### Acceptance Criteria
- Returns all point conditions
- Shows condition type, points value, thresholds

---

## US-PC-003: View Point Condition

**As a** household member
**I want to** view a specific point condition
**So that** I understand its rules

### Acceptance Criteria
- Shows all condition properties

---

## US-PC-004: Update Point Condition

**As a** household Owner or Admin
**I want to** modify a point condition
**So that** I can adjust the rules

### Acceptance Criteria
- Can modify points value
- Can modify thresholds and multipliers
- Can change task association

---

## US-PC-005: Delete Point Condition

**As a** household Owner or Admin
**I want to** remove a point condition
**So that** outdated rules are cleaned up

### Acceptance Criteria
- Point condition is removed
- Future events won't use this rule
