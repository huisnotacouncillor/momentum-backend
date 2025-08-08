-- Drop triggers first
DROP TRIGGER IF EXISTS update_comments_updated_at ON comments;
DROP TRIGGER IF EXISTS update_issues_updated_at ON issues;
DROP TRIGGER IF EXISTS update_projects_updated_at ON projects;
DROP TRIGGER IF EXISTS update_teams_updated_at ON teams;
DROP TRIGGER IF EXISTS update_workspaces_updated_at ON workspaces;

-- Drop function
DROP FUNCTION IF EXISTS update_updated_at_column();

-- Drop tables in reverse order (respecting foreign key constraints)
DROP TABLE IF EXISTS comments;
DROP TABLE IF EXISTS issue_labels;
DROP TABLE IF EXISTS issues;
DROP TABLE IF EXISTS labels;
DROP TABLE IF EXISTS cycles;
DROP TABLE IF EXISTS projects;
DROP TABLE IF EXISTS roadmaps;
DROP TABLE IF EXISTS team_members;
DROP TABLE IF EXISTS teams;
DROP TABLE IF EXISTS workspaces;

-- Drop ENUM types
DROP TYPE IF EXISTS issue_priority_enum;
DROP TYPE IF EXISTS issue_status_enum;
DROP TYPE IF EXISTS cycle_status_enum;
DROP TYPE IF EXISTS project_status_enum;

-- Drop extension
DROP EXTENSION IF EXISTS "uuid-ossp";
