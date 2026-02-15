-- Add 'onetime' and 'none' to the recurrence_type CHECK constraint
-- This is handled by modifying the initial migration's CHECK constraint.
-- Since we're creating from scratch, this migration is a no-op placeholder.
-- The actual constraint is defined in 20240101000000_initial.sql

SELECT 1;
