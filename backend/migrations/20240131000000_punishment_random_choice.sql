-- Add is_random_choice flag to punishments table
ALTER TABLE punishments ADD COLUMN is_random_choice BOOLEAN NOT NULL DEFAULT FALSE;

-- Create punishment_options table for linking random choice punishments to their options
CREATE TABLE punishment_options (
    id TEXT PRIMARY KEY NOT NULL,
    parent_punishment_id TEXT NOT NULL REFERENCES punishments(id) ON DELETE CASCADE,
    option_punishment_id TEXT NOT NULL REFERENCES punishments(id) ON DELETE CASCADE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(parent_punishment_id, option_punishment_id)
);

CREATE INDEX idx_punishment_options_parent ON punishment_options(parent_punishment_id);
