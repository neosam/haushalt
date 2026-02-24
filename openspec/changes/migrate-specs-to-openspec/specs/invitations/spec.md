## ADDED Requirements

### Requirement: Invite User to Household
Household Owners and Admins SHALL be able to invite users by email.

#### Scenario: Send invitation
- **WHEN** Owner/Admin invites user by email
- **THEN** invitation is created with expiration date
- **THEN** invitation is logged in activity log

#### Scenario: Specify role
- **WHEN** Owner/Admin specifies role in invitation
- **THEN** invitee will receive that role upon acceptance

#### Scenario: Default role
- **WHEN** no role is specified in invitation
- **THEN** invitee will receive Member role upon acceptance

---

### Requirement: List My Invitations
Authenticated users SHALL be able to see pending invitations to households.

#### Scenario: View pending invitations
- **WHEN** user requests their invitations
- **THEN** all pending invitations are returned
- **THEN** each shows household name, inviter, and offered role

---

### Requirement: List Household Invitations
Household Owners and Admins SHALL be able to see all invitations sent for their household.

#### Scenario: View household invitations
- **WHEN** Owner/Admin requests household invitations
- **THEN** all pending/sent invitations are returned
- **THEN** each shows invitee email, status, and sender

---

### Requirement: Accept Invitation
Invited users SHALL be able to accept household invitations.

#### Scenario: Accept invitation
- **WHEN** user accepts invitation
- **THEN** user joins household with assigned role
- **THEN** membership record is created
- **THEN** MemberJoined activity is logged
- **THEN** invitation status changes to Accepted

---

### Requirement: Decline Invitation
Invited users SHALL be able to decline household invitations.

#### Scenario: Decline invitation
- **WHEN** user declines invitation
- **THEN** invitation status changes to Declined
- **THEN** user does NOT join the household
- **THEN** invitation remains visible to household admins

---

### Requirement: Cancel Invitation
Household Owners and Admins SHALL be able to cancel pending invitations.

#### Scenario: Cancel pending invitation
- **WHEN** Owner/Admin cancels a pending invitation
- **THEN** invitation cannot be accepted

#### Scenario: Cannot cancel non-pending
- **WHEN** Owner/Admin attempts to cancel already-accepted invitation
- **THEN** request is rejected
