-- Revert changes to issues and projects tables

-- Remove indexes
DROP INDEX IF EXISTS idx_issues_team_id;
DROP INDEX IF EXISTS idx_issues_project_id;

-- Add back team_id column to projects table
ALTER TABLE projects ADD COLUMN team_id UUID NULL REFERENCES teams(id) ON DELETE SET NULL;

-- Restore project_id constraint and make it non-nullable
ALTER TABLE issues ALTER COLUMN project_id SET NOT NULL;

-- Remove team_id column from issues table
ALTER TABLE issues DROP COLUMN IF EXISTS team_id;