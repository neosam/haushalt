-- Journal entries table
CREATE TABLE IF NOT EXISTS journal_entries (
    id TEXT PRIMARY KEY NOT NULL,
    household_id TEXT NOT NULL REFERENCES households(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id),
    title TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL,
    entry_date DATE NOT NULL,
    is_shared BOOLEAN NOT NULL DEFAULT false,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Index for efficient querying by household (for shared entries)
CREATE INDEX IF NOT EXISTS idx_journal_entries_household_shared ON journal_entries(household_id, is_shared);

-- Index for efficient querying of private entries by user
CREATE INDEX IF NOT EXISTS idx_journal_entries_user_private ON journal_entries(household_id, user_id, is_shared);

-- Index for date-based browsing
CREATE INDEX IF NOT EXISTS idx_journal_entries_date ON journal_entries(household_id, entry_date);
