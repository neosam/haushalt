-- Add suggestion columns to tasks table
ALTER TABLE tasks ADD COLUMN suggestion TEXT CHECK(suggestion IN ('suggested', 'approved', 'denied'));
ALTER TABLE tasks ADD COLUMN suggested_by TEXT REFERENCES users(id);
