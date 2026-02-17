# Invitation User Stories

## US-INV-001: Invite User to Household

**As a** household Owner or Admin
**I want to** invite a user by email
**So that** they can join my household

### Acceptance Criteria
- Owner/Admin provides email address of invitee
- Optional role can be specified (defaults to Member)
- Invitation has an expiration date
- Invitation is logged in activity log

---

## US-INV-002: List My Invitations

**As an** authenticated user
**I want to** see pending invitations to households
**So that** I can decide whether to accept them

### Acceptance Criteria
- Shows all pending invitations for the user
- Shows household name and inviter information
- Shows the role being offered

---

## US-INV-003: List Household Invitations

**As a** household Owner or Admin
**I want to** see all invitations sent for my household
**So that** I can track pending invites

### Acceptance Criteria
- Shows all pending/sent invitations
- Shows invitee email and status
- Shows who sent the invitation

---

## US-INV-004: Accept Invitation

**As an** invited user
**I want to** accept a household invitation
**So that** I can join the household

### Acceptance Criteria
- User joins household with the assigned role
- Membership record is created
- MemberJoined activity is logged
- Invitation status changes to Accepted

---

## US-INV-005: Decline Invitation

**As an** invited user
**I want to** decline a household invitation
**So that** I don't join that household

### Acceptance Criteria
- Invitation status changes to Declined
- User does not join the household
- Invitation remains visible to household admins

---

## US-INV-006: Cancel Invitation

**As a** household Owner or Admin
**I want to** cancel a pending invitation
**So that** I can revoke invites that shouldn't have been sent

### Acceptance Criteria
- Only pending invitations can be cancelled
- Cancelled invitation cannot be accepted
- Owner/Admin can cancel any household invitation
