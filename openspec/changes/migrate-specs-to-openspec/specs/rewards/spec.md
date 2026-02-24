## ADDED Requirements

### Requirement: Create Reward
Household Owners and Admins SHALL be able to create rewards.

#### Scenario: Create basic reward
- **WHEN** Owner/Admin creates reward with name
- **THEN** reward is created

#### Scenario: Purchasable reward
- **WHEN** reward is marked as purchasable with point cost
- **THEN** members can buy it with points

#### Scenario: Requires confirmation
- **WHEN** requires_confirmation is set
- **THEN** redemptions need admin approval

---

### Requirement: List Household Rewards
Household members SHALL be able to see all available rewards.

#### Scenario: List rewards
- **WHEN** member requests rewards
- **THEN** all rewards are returned
- **THEN** name, description, cost are shown
- **THEN** purchasable and confirmation flags are shown

---

### Requirement: View Reward Details
Household members SHALL be able to view a specific reward.

#### Scenario: View reward
- **WHEN** member views reward
- **THEN** all reward properties are shown

---

### Requirement: Update Reward
Household Owners and Admins SHALL be able to modify rewards.

#### Scenario: Update reward
- **WHEN** Owner/Admin updates reward
- **THEN** name, description, cost can be modified
- **THEN** purchasable status can be changed
- **THEN** confirmation requirement can be changed

---

### Requirement: Delete Reward
Household Owners and Admins SHALL be able to remove rewards.

#### Scenario: Delete reward
- **WHEN** Owner/Admin deletes reward
- **THEN** reward is removed
- **THEN** task links are removed

---

### Requirement: Purchase Reward
Household members SHALL be able to purchase rewards with points.

#### Scenario: Purchase reward
- **WHEN** member purchases purchasable reward with sufficient points
- **THEN** points are deducted
- **THEN** UserReward entry is created

#### Scenario: Insufficient points
- **WHEN** member lacks sufficient points
- **THEN** purchase is rejected

---

### Requirement: Assign Reward to User
Household Owners and Admins SHALL be able to assign rewards without cost.

#### Scenario: Assign reward
- **WHEN** Owner/Admin assigns reward to user
- **THEN** no points are deducted
- **THEN** UserReward entry is created

---

### Requirement: Unassign Reward from User
Household Owners and Admins SHALL be able to remove rewards from users.

#### Scenario: Unassign reward
- **WHEN** Owner/Admin removes reward from user
- **THEN** UserReward entry is removed

---

### Requirement: View My Rewards
Household members SHALL be able to see their earned rewards.

#### Scenario: View my rewards
- **WHEN** member requests their rewards
- **THEN** all UserReward entries are returned
- **THEN** total amount, redeemed amount, pending amount are shown

---

### Requirement: View All User Rewards
Household Owners and Admins SHALL be able to see all members' rewards.

#### Scenario: View all user rewards
- **WHEN** Owner/Admin requests all user rewards
- **THEN** all UserReward entries in household are returned
- **THEN** shows which user has each reward

---

### Requirement: Redeem Reward
Household members SHALL be able to redeem earned rewards.

#### Scenario: Redeem with confirmation
- **WHEN** member redeems reward that requires confirmation
- **THEN** status is set to Pending

#### Scenario: Redeem without confirmation
- **WHEN** member redeems reward that doesn't require confirmation
- **THEN** status is set to Approved immediately
- **THEN** redeemed amount is incremented

---

### Requirement: View Pending Redemptions
Household Owners and Admins SHALL be able to see rewards awaiting confirmation.

#### Scenario: View pending
- **WHEN** Owner/Admin requests pending redemptions
- **THEN** all redemptions with Pending status are returned
- **THEN** shows who and which reward

---

### Requirement: Approve Redemption
Household Owners and Admins SHALL be able to approve pending redemptions.

#### Scenario: Approve redemption
- **WHEN** Owner/Admin approves redemption
- **THEN** status changes to Approved
- **THEN** redeemed amount is finalized
- **THEN** activity is logged

---

### Requirement: Reject Redemption
Household Owners and Admins SHALL be able to reject pending redemptions.

#### Scenario: Reject redemption
- **WHEN** Owner/Admin rejects redemption
- **THEN** status changes to Rejected
- **THEN** pending amount is removed
- **THEN** activity is logged

---

### Requirement: Link Reward to Task
Household Owners and Admins SHALL be able to attach rewards to tasks.

#### Scenario: Link reward
- **WHEN** Owner/Admin links reward to task with amount
- **THEN** completing task grants the reward

---

### Requirement: Unlink Reward from Task
Household Owners and Admins SHALL be able to remove rewards from tasks.

#### Scenario: Unlink reward
- **WHEN** Owner/Admin removes reward from task
- **THEN** future completions don't grant this reward

---

### Requirement: View Task Linked Rewards
Household members SHALL be able to see rewards attached to tasks.

#### Scenario: View task rewards
- **WHEN** member views task rewards
- **THEN** all linked rewards are shown
- **THEN** amount for each is shown

---

### Requirement: Create Random Choice Reward
Owners and Admins SHALL be able to create rewards with random selection.

#### Scenario: Create random choice reward
- **WHEN** reward is created with reward_type = random_choice
- **THEN** reward can have multiple options linked

#### Scenario: Minimum options
- **WHEN** random choice reward is created
- **THEN** at least 2 options must be linked

#### Scenario: Nested random choice
- **WHEN** random choice reward options are set
- **THEN** options can include other random choice rewards
- **THEN** self-reference is allowed

---

### Requirement: Link Reward Option
Owners and Admins SHALL be able to add options to random choice rewards.

#### Scenario: Link option
- **WHEN** Owner/Admin adds reward as option to random choice reward
- **THEN** link is created

---

### Requirement: Unlink Reward Option
Owners and Admins SHALL be able to remove options from random choice rewards.

#### Scenario: Unlink option
- **WHEN** Owner/Admin removes option from random choice reward
- **THEN** link is removed
- **THEN** random choice must still have at least 2 options

---

### Requirement: Pick Random Reward
Members with random choice rewards SHALL be able to randomly select one.

#### Scenario: Pick random
- **WHEN** member clicks "Pick one" on random choice reward
- **THEN** system randomly selects one option
- **THEN** selected reward is assigned to user
- **THEN** random choice assignment is marked resolved
- **THEN** activity is logged

#### Scenario: Nested random choice selected
- **WHEN** selected reward is also random choice
- **THEN** user must pick again
