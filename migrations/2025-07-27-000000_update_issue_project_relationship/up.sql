-- Update issues table to move from project-based to team-based issues
-- Add team_id column to issues table
ALTER TABLE issues ADD COLUMN team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE;

-- Remove project_id constraint and make it nullable
ALTER TABLE issues ALTER COLUMN project_id DROP NOT NULL;

-- Remove team_id column from projects table as projects can span multiple teams
ALTER TABLE projects DROP COLUMN IF EXISTS team_id;

-- Add indexes for better query performance
CREATE INDEX idx_issues_team_id ON issues(team_id);
CREATE INDEX idx_issues_project_id ON issues(project_id);