-- Add household invitations table for pending invitation system

CREATE TABLE IF NOT EXISTS household_invitations (
    id TEXT PRIMARY KEY NOT NULL,
    household_id TEXT NOT NULL REFERENCES households(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'member' CHECK(role IN ('admin', 'member')),
    invited_by TEXT NOT NULL REFERENCES users(id),
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'accepted', 'declined', 'expired')),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME NOT NULL,
    responded_at DATETIME
);

CREATE INDEX IF NOT EXISTS idx_invitations_household ON household_invitations(household_id);
CREATE INDEX IF NOT EXISTS idx_invitations_email ON household_invitations(email);
CREATE INDEX IF NOT EXISTS idx_invitations_status ON household_invitations(status);
CREATE UNIQUE INDEX IF NOT EXISTS idx_invitations_pending_unique ON household_invitations(household_id, email) WHERE status = 'pending';
