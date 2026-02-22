-- Solo Mode: Self-discipline feature where all users are treated as Members
-- with restricted permissions until exit via 48-hour cooldown

ALTER TABLE household_settings ADD COLUMN solo_mode BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE household_settings ADD COLUMN solo_mode_exit_requested_at DATETIME;
ALTER TABLE household_settings ADD COLUMN solo_mode_previous_hierarchy_type TEXT;
