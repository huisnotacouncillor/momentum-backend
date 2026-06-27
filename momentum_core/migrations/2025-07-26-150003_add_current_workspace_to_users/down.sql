-- Remove index on current_workspace_id
DROP INDEX IF EXISTS idx_users_current_workspace_id;

-- Remove current_workspace_id column from users table
ALTER TABLE users DROP COLUMN IF EXISTS current_workspace_id;