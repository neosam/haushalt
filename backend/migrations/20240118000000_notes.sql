-- Notes table for shared and private notes
CREATE TABLE IF NOT EXISTS notes (
    id TEXT PRIMARY KEY NOT NULL,
    household_id TEXT NOT NULL REFERENCES households(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id),
    title TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    is_shared BOOLEAN NOT NULL DEFAULT false,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Index for efficient querying of notes by household (for shared notes)
CREATE INDEX IF NOT EXISTS idx_notes_household_shared ON notes(household_id, is_shared);

-- Index for efficient querying of private notes by user
CREATE INDEX IF NOT EXISTS idx_notes_user_private ON notes(household_id, user_id, is_shared);
