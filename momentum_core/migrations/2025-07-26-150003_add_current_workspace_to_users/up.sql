-- Add current_workspace_id field to users table to track user's currently selected workspace
ALTER TABLE users ADD COLUMN current_workspace_id UUID NULL REFERENCES workspaces(id) ON DELETE SET NULL;

-- Create index for better query performance on current_workspace_id
CREATE INDEX idx_users_current_workspace_id ON users(current_workspace_id);