# Rewards User Stories

## US-REW-001: Create Reward

**As a** household Owner or Admin
**I want to** create a reward
**So that** members can earn or purchase it

### Acceptance Criteria
- Reward has a name
- Optional description
- Optional point cost (for purchasable rewards)
- Is purchasable flag (can members buy it with points)
- Requires confirmation flag (must admin approve redemption)

---

## US-REW-002: List Household Rewards

**As a** household member
**I want to** see all available rewards
**So that** I know what I can earn

### Acceptance Criteria
- Returns all rewards in the household
- Shows name, description, cost
- Shows if purchasable and if requires confirmation

---

## US-REW-003: View Reward Details

**As a** household member
**I want to** view a specific reward
**So that** I understand its requirements

### Acceptance Criteria
- Shows all reward properties

---

## US-REW-004: Update Reward

**As a** household Owner or Admin
**I want to** modify a reward
**So that** I can adjust its properties

### Acceptance Criteria
- Can modify name, description, cost
- Can change purchasable status
- Can change confirmation requirement

---

## US-REW-005: Delete Reward

**As a** household Owner or Admin
**I want to** remove a reward
**So that** outdated rewards are cleaned up

### Acceptance Criteria
- Reward is removed
- Task links to this reward are removed

---

## US-REW-006: Purchase Reward

**As a** household member
**I want to** purchase a reward with my points
**So that** I can redeem my earnings

### Acceptance Criteria
- Reward must be marked as purchasable
- User must have enough points
- Points are deducted from user
- UserReward entry is created

---

## US-REW-007: Assign Reward to User

**As a** household Owner or Admin
**I want to** assign a reward to a user without cost
**So that** I can reward them directly

### Acceptance Criteria
- No points are deducted
- UserReward entry is created
- Used for automatic task completion rewards

---

## US-REW-008: Unassign Reward from User

**As a** household Owner or Admin
**I want to** remove a reward from a user
**So that** I can correct mistakes

### Acceptance Criteria
- UserReward entry is removed
- User no longer has that reward instance

---

## US-REW-009: View My Rewards

**As a** household member
**I want to** see rewards I've earned or purchased
**So that** I can track what I have

### Acceptance Criteria
- Shows all my UserReward entries
- Shows total amount
- Shows redeemed amount
- Shows pending redemption amount

---

## US-REW-010: View All User Rewards

**As a** household Owner or Admin
**I want to** see all members' rewards
**So that** I can manage the reward system

### Acceptance Criteria
- Shows all UserReward entries in household
- Shows which user has each reward

---

## US-REW-011: Redeem Reward

**As a** household member
**I want to** redeem a reward I've earned
**So that** I can claim my prize

### Acceptance Criteria
- If requires_confirmation: status = Pending
- Otherwise: status = Approved immediately
- Redeemed amount is incremented

---

## US-REW-012: View Pending Redemptions

**As a** household Owner or Admin
**I want to** see rewards awaiting confirmation
**So that** I can approve or reject them

### Acceptance Criteria
- Returns all redemptions with status = Pending
- Shows who is redeeming
- Shows which reward

---

## US-REW-013: Approve Redemption

**As a** household Owner or Admin
**I want to** approve a pending redemption
**So that** the user can receive their reward

### Acceptance Criteria
- Redemption status changes to Approved
- Redeemed amount is finalized
- Activity is logged

---

## US-REW-014: Reject Redemption

**As a** household Owner or Admin
**I want to** reject a pending redemption
**So that** invalid redemptions are denied

### Acceptance Criteria
- Redemption status changes to Rejected
- Pending amount is removed
- Activity is logged

---

## US-REW-015: Link Reward to Task

**As a** household Owner or Admin
**I want to** attach a reward to a task
**So that** completing the task automatically grants the reward

### Acceptance Criteria
- Reward is linked to task
- Amount can be specified (how many times to apply)
- Reward is granted when task is completed

---

## US-REW-016: Unlink Reward from Task

**As a** household Owner or Admin
**I want to** remove a reward from a task
**So that** it's no longer granted automatically

### Acceptance Criteria
- Reward link is removed
- Future completions don't grant this reward

---

## US-REW-017: View Task Linked Rewards

**As a** household member
**I want to** see rewards attached to a task
**So that** I know what I'll earn

### Acceptance Criteria
- Shows all rewards linked to the task
- Shows the amount for each
