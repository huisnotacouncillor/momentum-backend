-- Add back status column to issues table (rollback migration)
ALTER TABLE issues ADD COLUMN status TEXT NOT NULL DEFAULT 'todo';