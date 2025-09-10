-- Remove status column from issues table since we now use workflow_state_id
ALTER TABLE issues DROP COLUMN status;