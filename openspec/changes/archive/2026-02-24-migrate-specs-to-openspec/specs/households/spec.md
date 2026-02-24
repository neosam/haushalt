## ADDED Requirements

### Requirement: Create Household
Authenticated users SHALL be able to create new households.

#### Scenario: Create household
- **WHEN** user provides household name
- **THEN** household is created
- **THEN** creator becomes Owner
- **THEN** default settings are applied

---

### Requirement: List My Households
Authenticated users SHALL be able to see all households they belong to.

#### Scenario: List households
- **WHEN** user requests their households
- **THEN** all households where user is member are returned
- **THEN** each shows household name and user's role

---

### Requirement: View Household Details
Household members SHALL be able to view household information.

#### Scenario: Member views details
- **WHEN** member requests household details
- **THEN** household name and basic info are returned

#### Scenario: Non-member access denied
- **WHEN** non-member requests household details
- **THEN** access is denied

---

### Requirement: Update Household Name
Household Owners and Admins SHALL be able to rename the household.

#### Scenario: Owner renames household
- **WHEN** Owner submits new household name
- **THEN** household name is updated
- **THEN** name updates across all views immediately

#### Scenario: Admin renames household
- **WHEN** Admin submits new household name
- **THEN** household name is updated

#### Scenario: Member cannot rename
- **WHEN** Member attempts to rename household
- **THEN** request is rejected

#### Scenario: Invalid name rejected
- **WHEN** user submits empty or whitespace-only name
- **THEN** request is rejected with validation error

---

### Requirement: Delete Household
Household Owners SHALL be able to delete the household.

#### Scenario: Owner deletes household
- **WHEN** Owner deletes household
- **THEN** all associated data is removed
- **THEN** members lose access

#### Scenario: Non-owner cannot delete
- **WHEN** Admin or Member attempts to delete household
- **THEN** request is rejected

---

### Requirement: List Household Members
Household members SHALL be able to see all members in the household.

#### Scenario: View members
- **WHEN** member requests household members
- **THEN** all members are returned with usernames
- **THEN** each member's role is shown
- **THEN** each member's current points are shown

---

### Requirement: Adjust Member Points
Household Owners and Admins SHALL be able to manually adjust member points.

#### Scenario: Add points
- **WHEN** Owner/Admin adds points to member with reason
- **THEN** member's points increase
- **THEN** change is logged in activity log

#### Scenario: Deduct points
- **WHEN** Owner/Admin deducts points from member with reason
- **THEN** member's points decrease
- **THEN** change is logged in activity log

#### Scenario: Reason required
- **WHEN** Owner/Admin adjusts points without reason
- **THEN** request is rejected

---

### Requirement: Update Member Role
Household Owners SHALL be able to change member roles.

#### Scenario: Change role
- **WHEN** Owner changes member's role
- **THEN** member's role is updated
- **THEN** change is logged in activity log

#### Scenario: Available roles
- **WHEN** Owner changes role
- **THEN** available roles are Owner, Admin, Member

#### Scenario: Non-owner cannot change roles
- **WHEN** Admin or Member attempts to change roles
- **THEN** request is rejected

---

### Requirement: Remove Member
Household Owners and Admins SHALL be able to remove members.

#### Scenario: Owner removes any member
- **WHEN** Owner removes a member (not themselves)
- **THEN** member loses access immediately

#### Scenario: Admin removes Member
- **WHEN** Admin removes a Member
- **THEN** member loses access immediately

#### Scenario: Admin cannot remove Admin/Owner
- **WHEN** Admin attempts to remove Admin or Owner
- **THEN** request is rejected

---

### Requirement: View Leaderboard
Household members SHALL be able to see a ranked list of members by points.

#### Scenario: View leaderboard
- **WHEN** member requests leaderboard
- **THEN** members are ranked by total points
- **THEN** shows rank position, username, points
- **THEN** shows tasks completed count
- **THEN** shows current streak

---

### Requirement: Get Household Settings
Household Owners and Admins SHALL be able to view household settings.

#### Scenario: View settings
- **WHEN** Owner/Admin requests settings
- **THEN** dark mode setting is shown
- **THEN** custom role labels are shown
- **THEN** hierarchy type is shown
- **THEN** timezone is shown
- **THEN** feature flags are shown (rewards, punishments, chat)
- **THEN** vacation mode status and dates are shown
- **THEN** task defaults are shown

---

### Requirement: Update Household Settings
Household Owners SHALL be able to configure household settings.

#### Scenario: Toggle dark mode
- **WHEN** Owner toggles dark mode
- **THEN** dark mode setting is updated

#### Scenario: Set custom role labels
- **WHEN** Owner sets custom labels for roles
- **THEN** role labels are updated

#### Scenario: Choose hierarchy type
- **WHEN** Owner selects hierarchy type (Equals/Organized/Hierarchy)
- **THEN** hierarchy type is updated
- **THEN** permissions reflect new hierarchy

#### Scenario: Set timezone
- **WHEN** Owner sets timezone
- **THEN** timezone is updated

#### Scenario: Toggle features
- **WHEN** Owner toggles rewards/punishments/chat
- **THEN** feature availability is updated

#### Scenario: Configure vacation mode
- **WHEN** Owner enables vacation mode with optional dates
- **THEN** vacation mode is active
- **THEN** tasks are paused during vacation

---

### Requirement: View Household Overview Page
Household members SHALL see an overview page with pending items and tasks.

#### Scenario: View overview
- **WHEN** member views household overview
- **THEN** household name and navigation tabs are shown
- **THEN** active announcement banner is shown (if any)
- **THEN** leaderboard section is shown

#### Scenario: Pending items for managers
- **WHEN** Owner/Admin views overview
- **THEN** pending reviews section is shown (if any exist)
- **THEN** pending suggestions section is shown (if any exist)
- **THEN** pending confirmations section is shown (if any exist)

#### Scenario: Empty pending sections hidden
- **WHEN** there are no pending items of a type
- **THEN** that section is hidden

#### Scenario: Task list
- **WHEN** member views overview
- **THEN** today's tasks are shown grouped by status
- **THEN** assignment filter is available

---

### Requirement: Household Navigation Tabs
The tab navigation SHALL maintain scroll position across tab switches on mobile.

#### Scenario: Preserve scroll position
- **WHEN** user switches between tabs
- **THEN** tab bar scroll position is preserved
- **THEN** tab highlighting updates without full re-render

#### Scenario: Conditional tabs appear
- **WHEN** settings are loaded with features enabled
- **THEN** Rewards, Punishments, Chat tabs appear accordingly

#### Scenario: Translated labels
- **WHEN** user has language preference set
- **THEN** all tab labels are translated correctly

---

### Requirement: Vacation Mode Banner
The system SHALL display a banner when household is in vacation mode.

#### Scenario: Banner displayed
- **WHEN** vacation mode is active
- **THEN** banner is shown at top of household view
- **THEN** banner shows vacation status

#### Scenario: End date shown
- **WHEN** vacation end date is set
- **THEN** banner shows when vacation ends

#### Scenario: All members see banner
- **WHEN** vacation mode is active
- **THEN** banner is visible to all household members

---

### Requirement: Task Defaults
Household Owners SHALL be able to configure default values for new tasks.

#### Scenario: Configure default points
- **WHEN** Owner sets default points for completion/miss
- **THEN** new tasks are pre-filled with these values

#### Scenario: Configure default rewards
- **WHEN** Owner sets default rewards with amounts
- **THEN** new tasks are pre-populated with these rewards

#### Scenario: Configure default punishments
- **WHEN** Owner sets default punishments with amounts
- **THEN** new tasks are pre-populated with these punishments

#### Scenario: Override defaults
- **WHEN** user creates task with default values
- **THEN** user can modify or remove defaults before saving

#### Scenario: Deleted reward/punishment removed
- **WHEN** default reward/punishment is deleted from household
- **THEN** it is automatically removed from defaults

---

### Requirement: Solo Mode
Household Owners SHALL be able to activate Solo Mode for self-discipline.

#### Scenario: Activate Solo Mode
- **WHEN** Owner activates Solo Mode with confirmation
- **THEN** Solo Mode starts immediately
- **THEN** all users are treated as Members

#### Scenario: Restricted permissions
- **WHEN** Solo Mode is active
- **THEN** no one can create/edit/delete tasks directly
- **THEN** no one can modify household settings
- **THEN** no one can change roles or adjust points

#### Scenario: Task suggestions auto-accepted
- **WHEN** task is suggested during Solo Mode
- **THEN** task is automatically approved
- **THEN** rewards/punishments are overwritten with household defaults

#### Scenario: Deferred task scheduling
- **WHEN** user creates task without specific date in Solo Mode
- **THEN** task appears in backlog (No Schedule section)
- **THEN** task does not trigger penalties
- **THEN** user can later set date via context menu

#### Scenario: Exit via cooldown
- **WHEN** member requests to exit Solo Mode
- **THEN** 48-hour cooldown period starts
- **THEN** Solo Mode remains active during cooldown
- **THEN** after 48 hours, Solo Mode deactivates automatically

#### Scenario: Cancel exit request
- **WHEN** member cancels exit request during cooldown
- **THEN** cooldown is interrupted
- **THEN** Solo Mode continues

#### Scenario: Solo Mode banner
- **WHEN** Solo Mode is active
- **THEN** banner shows status and exit options on all household pages
