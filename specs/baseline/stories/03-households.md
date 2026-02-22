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

## US-HH-004: Update Household (Rename)

> **Status:** Implemented
> **Implemented:** 2026-02-21

**As a** household Owner or Admin
**I want to** rename the household
**So that** I can keep household information accurate

### Acceptance Criteria

#### Permissions
- Owner can rename household
- Admin can rename household
- Members cannot rename household

#### UI Access
- Rename option available in Household Settings page
- Only visible to users with rename permission (Owner/Admin)

#### Rename Flow
1. User navigates to Household Settings
2. User sees current household name with an "Edit" or "Rename" button
3. Clicking opens an inline edit or modal with text input
4. User enters new name and confirms
5. Household name updates immediately across all views
6. Success message is shown

#### Validation
- Name cannot be empty
- Name has reasonable max length (e.g., 100 characters)
- Whitespace-only names are rejected

#### UI Updates After Rename
- Household name in navigation/tabs updates
- Dashboard household list updates
- Household overview header updates

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
- Shows vacation mode status and dates
- Shows task defaults (see US-HH-016)

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
- Can enable/disable vacation mode
- Can set vacation start and end dates
- Can configure task defaults (see US-HH-016)

---

## US-HH-013: View Household Overview Page

**As a** household member
**I want to** see an overview of the household
**So that** I can quickly see status and pending items

### Acceptance Criteria
- Shows household name and navigation tabs
- Shows active announcement banner (if any)
- Shows today's tasks grouped by household
- Shows leaderboard section
- Shows Pending Reviews section (task completions awaiting approval)
  - Only visible to managers (Owner/Admin)
  - **Hidden when there are no pending reviews**
- Shows Pending Confirmations section (reward redemptions and punishment completions awaiting confirmation)
  - Only visible to managers (Owner/Admin)
  - **Hidden when there are no pending confirmations**
  - **Hidden when rewards/punishments features are disabled** (treats "feature not enabled" errors as empty)

---

## US-HH-014: Household Navigation Tabs

**As a** household member on a mobile device
**I want** the navigation tabs to maintain their scroll position when switching tabs
**So that** I don't lose my place in the tab list when navigating

### Acceptance Criteria
- Tab navigation bar is horizontally scrollable on mobile
- When clicking a tab, the tab bar must NOT re-render/redraw
- Scroll position of the tab bar is preserved across tab switches
- Tab highlighting updates without full component re-render
- **Conditional tabs (Rewards, Punishments, Chat) must appear when settings are loaded**
- **All tab labels must be translated correctly in the user's language**

### Technical Notes
- Use stable component keys to prevent re-rendering
- Consider using CSS-only active state changes where possible
- The `HouseholdTabs` component should receive stable props to avoid unnecessary re-renders
- **IMPORTANT: Follow Reactive Data Pattern (Constitution 14.3)**
  - Settings data is loaded asynchronously
  - Component must react to settings updates, not capture initial `None` value
  - Pass `RwSignal<Option<HouseholdSettings>>` instead of unwrapped value
  - Or wrap component in reactive closure `{move || ...}`

### Known Issues (Fixed)

#### Bug: Conditional Tabs Not Appearing
- **Symptom:** Rewards, Punishments, Chat tabs missing even when features enabled
- **Cause:** `HouseholdTabs` received `settings.get()` (unwrapped value) instead of signal
- **Root Cause:** Props were evaluated once at component creation, before API response
- **Fix:** Pass reactive signal or wrap in `move ||` closure

#### Bug: English Labels in Non-English UI
- **Symptom:** Tab labels shown in English despite German UI setting
- **Cause:** Same reactivity issue - i18n evaluated before translations loaded
- **Fix:** Ensure i18n access is inside reactive context

---

## US-HH-015: Vacation Mode Banner

> **Status:** Implemented
> **Implemented:** 2026-02-20

**As a** household member
**I want to** see a banner when the household is in vacation mode
**So that** I understand why tasks are paused and know when vacation ends

### Acceptance Criteria
- Banner is displayed at the top of the household view when vacation mode is active
- Banner shows that the household is on vacation
- If vacation end date is set, banner shows when vacation ends
- Banner is visually distinct (e.g., info/warning style)
- Banner is visible to all household members

---

## US-HH-016: Task Defaults

**As a** household Owner
**I want to** configure default values for new tasks
**So that** I don't have to set the same values repeatedly when creating tasks

### Acceptance Criteria

#### Settings Available
- Default points on completion (optional, can be empty for "none")
- Default points deducted on miss (optional, can be empty for "none")
- Default rewards on completion (list of rewards with amounts, can be empty)
- Default punishments on miss (list of punishments with amounts, can be empty)

#### UI Access
- Configured in Household Settings page
- Only visible/editable by Owner
- Located in a "Task Defaults" section

#### Default Rewards/Punishments Management
- UI similar to TaskModal reward/punishment selection:
  - Dropdown to select reward/punishment to add
  - Amount input field (default: 1)
  - "Add" button to add to the list
  - List shows currently configured defaults with remove buttons
- Can add multiple rewards/punishments as defaults
- Each default has an associated amount (how many times it's applied)

#### Behavior
- When creating a new task, these defaults are pre-filled:
  - Points fields are populated with default values
  - Default rewards are pre-selected in the rewards list
  - Default punishments are pre-selected in the punishments list
- User can override defaults when creating/editing individual tasks
- User can add more rewards/punishments beyond the defaults
- User can remove pre-selected defaults before saving
- If a default reward/punishment is deleted from household, it's automatically removed from defaults
- Defaults only apply to new tasks, not existing ones

#### Validation
- Points values must be non-negative integers (or empty)
- Selected rewards must exist and be active in the household
- Selected punishments must exist and be active in the household
- Amount for each reward/punishment must be at least 1

#### Display in Settings
- Shows current default points (or "None" if not set)
- Shows list of default rewards with amounts (or "None configured" if empty)
- Shows list of default punishments with amounts (or "None configured" if empty)
- Reward/punishment sections only show if respective feature is enabled

---

## US-HH-017: Solo Mode

> **Status:** Planned

**As a** household Owner
**I want to** activate Solo Mode for the household
**So that** I can give myself tasks without being able to easily ignore them

### Overview

Solo Mode is a self-discipline feature where the user voluntarily gives up control over the household. All members (including Owner/Admin) are treated as regular Members with restricted permissions. This prevents "cheating" by modifying or deleting tasks.

### Acceptance Criteria

#### Activation
- Only Owner can activate Solo Mode
- Activation requires confirmation dialog ("Are you sure?")
- Solo Mode starts immediately upon confirmation
- No end date is set - runs indefinitely until exit via cooldown
- Activation option available in Household Settings (only when Solo Mode is NOT active)

#### Permissions During Solo Mode
- **All users** are treated like Members in Hierarchy mode
- No one can:
  - Create regular tasks (only suggest)
  - Edit existing tasks
  - Delete tasks
  - Modify household settings
  - Change member roles
  - Adjust points manually
- Everyone can:
  - Complete assigned tasks
  - Suggest new tasks
  - View all household information
  - Redeem rewards (if enabled)

#### Task Suggestions in Solo Mode
- All new tasks are created as suggestions
- Suggestions are **automatically accepted** (status: 'approved')
- Rewards/Punishments on auto-accepted tasks are **overwritten** with household defaults:
  - `points_reward` → `default_points_reward` from settings
  - `points_penalty` → `default_points_penalty` from settings
  - Task rewards → `default_rewards` from settings
  - Task punishments → `default_punishments` from settings
- Existing tasks remain unchanged (keep their original rewards/punishments)

#### Exiting Solo Mode (Cooldown)
- Exit is initiated via the **Solo Mode Banner** (not via Settings)
- Any household member can request to exit Solo Mode
- Exit is **not immediate** - a 48-hour cooldown period starts
- During cooldown:
  - Solo Mode remains fully active
  - Banner shows countdown: "Solo Mode ends in X hours"
  - Button to **cancel** the exit request (cooldown is interrupted)
- After 48 hours: Solo Mode automatically deactivates
- Household returns to previous hierarchy type and normal permissions

#### Solo Mode Banner
- Displayed on all household pages when Solo Mode is active
- **When active (no exit requested):**
  - Text: "Solo Mode active - restricted permissions"
  - Button: "Request Exit"
- **During cooldown:**
  - Text: "Solo Mode ends in [HH:MM:SS]" (countdown)
  - Button: "Cancel Exit"
- Banner is visible to all household members

#### Data Model
New fields in `household_settings`:
- `solo_mode: bool` - Whether Solo Mode is active
- `solo_mode_exit_requested_at: Option<DateTime<Utc>>` - When exit was requested (null = no exit pending)
- `solo_mode_previous_hierarchy_type: Option<HierarchyType>` - To restore after exit

#### Settings Page During Solo Mode
- Settings page is accessible but read-only
- Shows "Solo Mode active" status prominently
- All controls are disabled/hidden
- No "Activate Solo Mode" button (already active)

#### Edge Cases
- If Owner tries to leave household during Solo Mode: Not allowed
- If all members leave: Solo Mode is automatically deactivated
- Vacation mode cannot be toggled during Solo Mode
- Hierarchy type setting is hidden/disabled during Solo Mode

### Technical Notes
- Solo Mode check should be at the service layer, not just handlers
- Permission checks: `is_solo_mode_active()` should override `can_manage()` results
- Background job needed to check `solo_mode_exit_requested_at` and deactivate after 48h
- Cooldown duration: 48 hours (fixed, not configurable initially)
