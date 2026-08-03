-- Per-report switch: pull OneTime / free-form ("no fixed date") tasks out of the report's
-- "Due today" section into their own "No fixed date" section.
--
-- DEFAULT 0 keeps existing reports exactly as they were — undated tasks stay mixed into
-- "Due today" until a report explicitly opts in.
ALTER TABLE public_reports ADD COLUMN separate_undated INTEGER NOT NULL DEFAULT 0;
