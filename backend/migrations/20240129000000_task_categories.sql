-- Task categories table
CREATE TABLE task_categories (
    id TEXT PRIMARY KEY NOT NULL,
    household_id TEXT NOT NULL REFERENCES households(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(household_id, name)
);

CREATE INDEX idx_task_categories_household ON task_categories(household_id);

-- Add category_id to tasks table
ALTER TABLE tasks ADD COLUMN category_id TEXT REFERENCES task_categories(id) ON DELETE SET NULL;

CREATE INDEX idx_tasks_category ON tasks(category_id);
