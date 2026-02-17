# Activity Log User Stories

## US-ACT-001: View Activity Logs

**As a** household member
**I want to** see household activity history
**So that** I can track what's happening

### Acceptance Criteria
- Owner sees all activities
- Non-owners see only their own activities
- Can limit number of entries returned
- Sorted by most recent first

---

## US-ACT-002: Activity Types Tracked

**As a** household member
**I want** activities to be automatically logged
**So that** there's an audit trail

### Acceptance Criteria
Activities tracked:
- Task: Created, Updated, Deleted, Assigned
- Task: Completed, Missed
- Task: CompletionApproved, CompletionRejected
- Reward: Created, Deleted, Assigned
- Reward: Purchased, Redeemed
- Reward: RedemptionApproved, RedemptionRejected
- Punishment: Created, Deleted, Assigned
- Punishment: Completed
- Punishment: CompletionApproved, CompletionRejected
- Points: Adjusted
- Member: Joined, Left, RoleChanged
- Invitation: Sent
- Settings: Changed
