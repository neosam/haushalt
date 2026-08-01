-- Per-report switch for the "Missed yesterday" section (Phase 6 follow-up).
--
-- Defaults to 1 so every existing report keeps rendering exactly as before.
ALTER TABLE public_reports ADD COLUMN include_missed INTEGER NOT NULL DEFAULT 1;
