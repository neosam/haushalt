## ADDED Requirements

### Requirement: View Activity Logs
Household members SHALL be able to view household activity history.

#### Scenario: Owner views all activities
- **WHEN** Owner requests activity logs
- **THEN** all household activities are returned

#### Scenario: Non-owner views own activities
- **WHEN** non-Owner requests activity logs
- **THEN** only their own activities are returned

#### Scenario: Pagination
- **WHEN** activity logs are requested with limit
- **THEN** limited number of entries is returned
- **THEN** sorted by most recent first

---

### Requirement: Activity Types Tracked
The system SHALL automatically log household activities.

#### Scenario: Task activities
- **WHEN** task is created, updated, deleted, or assigned
- **THEN** activity is logged

#### Scenario: Task completion activities
- **WHEN** task is completed, missed, approved, or rejected
- **THEN** activity is logged

#### Scenario: Reward activities
- **WHEN** reward is created, deleted, assigned, purchased, or redeemed
- **THEN** activity is logged

#### Scenario: Reward approval activities
- **WHEN** reward redemption is approved or rejected
- **THEN** activity is logged

#### Scenario: Punishment activities
- **WHEN** punishment is created, deleted, assigned, or completed
- **THEN** activity is logged

#### Scenario: Punishment approval activities
- **WHEN** punishment completion is approved or rejected
- **THEN** activity is logged

#### Scenario: Points activities
- **WHEN** points are manually adjusted
- **THEN** activity is logged

#### Scenario: Member activities
- **WHEN** member joins, leaves, or role changes
- **THEN** activity is logged

#### Scenario: Invitation activities
- **WHEN** invitation is sent
- **THEN** activity is logged

#### Scenario: Settings activities
- **WHEN** settings are changed
- **THEN** activity is logged
