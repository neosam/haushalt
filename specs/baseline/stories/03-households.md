# Household User Stories

## US-HH-001: Create Household

**As an** authenticated user
**I want to** create a new household
**So that** I can organize tasks and collaborate with others

### Acceptance Criteria
- User provides a household name
- Creator automatically becomes the Owner
- Household is created with default settings

---

## US-HH-002: List My Households

**As an** authenticated user
**I want to** see all households I belong to
**So that** I can navigate between them

### Acceptance Criteria
- Returns all households where user is a member
- Shows household name and user's role in each

---

## US-HH-003: View Household Details

**As a** household member
**I want to** view household information
**So that** I can see the household name and details

### Acceptance Criteria
- Member can view household name and basic info
- Non-members cannot access household details

---

## US-HH-004: Update Household

**As a** household Owner or Admin
**I want to** edit the household name
**So that** I can keep household information accurate

### Acceptance Criteria
- Owner can update household name
- Admin can update household name
- Members cannot update household

---

## US-HH-005: Delete Household

**As a** household Owner
**I want to** delete the household
**So that** I can remove it when no longer needed

### Acceptance Criteria
- Only Owner can delete the household
- All associated data is removed
- Members are notified/removed

---

## US-HH-006: List Household Members

**As a** household member
**I want to** see all members in the household
**So that** I know who I'm collaborating with

### Acceptance Criteria
- Shows all members with their usernames
- Shows each member's role (Owner, Admin, Member)
- Shows each member's current points

---

## US-HH-007: Adjust Member Points

**As a** household Owner or Admin
**I want to** manually add or deduct points from a member
**So that** I can correct errors or reward/penalize behavior

### Acceptance Criteria
- Owner/Admin can add positive or negative points
- A reason must be provided
- Change is logged in activity log

---

## US-HH-008: Update Member Role

**As a** household Owner
**I want to** change a member's role
**So that** I can grant or revoke permissions

### Acceptance Criteria
- Only Owner can change roles
- Roles available: Owner, Admin, Member
- Role change is logged in activity log

---

## US-HH-009: Remove Member

**As a** household Owner or Admin
**I want to** remove a member from the household
**So that** they no longer have access

### Acceptance Criteria
- Owner can remove any member (except themselves)
- Admin can remove Members only
- Removed member loses access immediately

---

## US-HH-010: View Leaderboard

**As a** household member
**I want to** see a ranked list of members by points
**So that** I can see competition standings

### Acceptance Criteria
- Members are ranked by total points
- Shows rank position, username, points
- Shows tasks completed count
- Shows current streak

---

## US-HH-011: Get Household Settings

**As a** household Owner or Admin
**I want to** view household settings
**So that** I can see current configuration

### Acceptance Criteria
- Shows dark mode setting
- Shows custom role labels (Owner, Admin, Member)
- Shows hierarchy type (Equals, Organized, Hierarchy)
- Shows timezone configuration
- Shows feature flags (rewards_enabled, punishments_enabled, chat_enabled)

---

## US-HH-012: Update Household Settings

**As a** household Owner
**I want to** configure household settings
**So that** I can customize how the household operates

### Acceptance Criteria
- Can toggle dark mode
- Can set custom labels for roles
- Can choose hierarchy type:
  - Equals: Everyone can manage tasks/rewards/punishments
  - Organized: Only Owner/Admin can manage
  - Hierarchy: Only Owner/Admin manage; only Members can be assigned
- Can set timezone
- Can enable/disable rewards feature
- Can enable/disable punishments feature
- Can enable/disable chat feature
