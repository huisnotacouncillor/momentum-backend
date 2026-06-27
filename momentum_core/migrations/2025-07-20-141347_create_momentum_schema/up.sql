-- Momentum Project Management System Database Schema
-- Based on the design document

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create ENUM types
CREATE TYPE project_status_enum AS ENUM ('planned', 'active', 'paused', 'completed', 'canceled');
CREATE TYPE cycle_status_enum AS ENUM ('planned', 'active', 'completed');
CREATE TYPE issue_status_enum AS ENUM ('backlog', 'todo', 'in_progress', 'in_review', 'done', 'canceled');
CREATE TYPE issue_priority_enum AS ENUM ('none', 'low', 'medium', 'high', 'urgent');

-- Create workspaces table
CREATE TABLE workspaces (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    url_key VARCHAR(255) NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create teams table
CREATE TABLE teams (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    team_key VARCHAR(10) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create team_members table (many-to-many relationship between users and teams)
CREATE TABLE team_members (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, team_id)
);

-- Create roadmaps table
CREATE TABLE roadmaps (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    start_date DATE NOT NULL,
    end_date DATE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create projects table
CREATE TABLE projects (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    team_id UUID REFERENCES teams(id) ON DELETE SET NULL,
    roadmap_id UUID REFERENCES roadmaps(id) ON DELETE SET NULL,
    owner_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    name VARCHAR(255) NOT NULL,
    project_key VARCHAR(10) NOT NULL,
    description TEXT,
    status project_status_enum NOT NULL DEFAULT 'planned',
    target_date DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(workspace_id, project_key)
);

-- Create cycles table
CREATE TABLE cycles (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    start_date DATE NOT NULL,
    end_date DATE NOT NULL,
    status cycle_status_enum NOT NULL DEFAULT 'planned',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create labels table
CREATE TABLE labels (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    color VARCHAR(7),
    UNIQUE(workspace_id, name)
);

-- Create issues table
CREATE TABLE issues (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    cycle_id UUID REFERENCES cycles(id) ON DELETE SET NULL,
    creator_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    assignee_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    parent_issue_id UUID REFERENCES issues(id) ON DELETE SET NULL,
    issue_number SERIAL,
    title VARCHAR(512) NOT NULL,
    description TEXT,
    status issue_status_enum NOT NULL DEFAULT 'backlog',
    priority issue_priority_enum NOT NULL DEFAULT 'none',
    is_changelog_candidate BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create issue_labels table (many-to-many relationship between issues and labels)
CREATE TABLE issue_labels (
    issue_id UUID NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    label_id UUID NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
    PRIMARY KEY (issue_id, label_id)
);

-- Create comments table
CREATE TABLE comments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    issue_id UUID NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    author_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create indexes for performance optimization
-- Foreign key indexes
CREATE INDEX idx_teams_workspace_id ON teams(workspace_id);
CREATE INDEX idx_team_members_user_id ON team_members(user_id);
CREATE INDEX idx_team_members_team_id ON team_members(team_id);
CREATE INDEX idx_roadmaps_workspace_id ON roadmaps(workspace_id);
CREATE INDEX idx_projects_workspace_id ON projects(workspace_id);
CREATE INDEX idx_projects_team_id ON projects(team_id);
CREATE INDEX idx_projects_roadmap_id ON projects(roadmap_id);
CREATE INDEX idx_projects_owner_id ON projects(owner_id);
CREATE INDEX idx_cycles_team_id ON cycles(team_id);
CREATE INDEX idx_labels_workspace_id ON labels(workspace_id);
CREATE INDEX idx_issues_project_id ON issues(project_id);
CREATE INDEX idx_issues_cycle_id ON issues(cycle_id);
CREATE INDEX idx_issues_creator_id ON issues(creator_id);
CREATE INDEX idx_issues_assignee_id ON issues(assignee_id);
CREATE INDEX idx_issues_parent_issue_id ON issues(parent_issue_id);
CREATE INDEX idx_issue_labels_issue_id ON issue_labels(issue_id);
CREATE INDEX idx_issue_labels_label_id ON issue_labels(label_id);
CREATE INDEX idx_comments_issue_id ON comments(issue_id);
CREATE INDEX idx_comments_author_id ON comments(author_id);

-- High-frequency query indexes
CREATE INDEX idx_issues_status ON issues(status);
CREATE INDEX idx_issues_assignee_id_status ON issues(assignee_id, status);
CREATE INDEX idx_issues_cycle_id_status ON issues(cycle_id, status);
CREATE INDEX idx_issues_project_id_status ON issues(project_id, status);
CREATE INDEX idx_issues_created_at ON issues(created_at);
CREATE INDEX idx_issues_updated_at ON issues(updated_at);

-- Unique constraint indexes (already created by UNIQUE constraints)
-- users(email) - already exists
-- workspaces(url_key) - already exists
-- projects(workspace_id, project_key) - already exists
-- labels(workspace_id, name) - already exists

-- Create triggers for updated_at timestamps
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_workspaces_updated_at BEFORE UPDATE ON workspaces
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_teams_updated_at BEFORE UPDATE ON teams
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_projects_updated_at BEFORE UPDATE ON projects
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_issues_updated_at BEFORE UPDATE ON issues
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_comments_updated_at BEFORE UPDATE ON comments
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Insert sample data for testing
INSERT INTO workspaces (name, url_key) VALUES
('Momentum Demo', 'momentum-demo'),
('Acme Corp', 'acme-corp');

INSERT INTO teams (workspace_id, name, team_key) VALUES
((SELECT id FROM workspaces WHERE url_key = 'momentum-demo'), 'Engineering', 'ENG'),
((SELECT id FROM workspaces WHERE url_key = 'momentum-demo'), 'Design', 'DES'),
((SELECT id FROM workspaces WHERE url_key = 'acme-corp'), 'Product', 'PROD');

INSERT INTO roadmaps (workspace_id, name, description, start_date, end_date) VALUES
((SELECT id FROM workspaces WHERE url_key = 'momentum-demo'), 'Q3 2025', 'Third quarter roadmap', '2025-07-01', '2025-09-30'),
((SELECT id FROM workspaces WHERE url_key = 'momentum-demo'), 'Q4 2025', 'Fourth quarter roadmap', '2025-10-01', '2025-12-31');

INSERT INTO projects (workspace_id, team_id, roadmap_id, owner_id, name, project_key, description, status) VALUES
((SELECT id FROM workspaces WHERE url_key = 'momentum-demo'),
 (SELECT id FROM teams WHERE team_key = 'ENG'),
 (SELECT id FROM roadmaps WHERE name = 'Q3 2025'),
 1, 'Momentum Core', 'MOM', 'Core project management features', 'active'),
((SELECT id FROM workspaces WHERE url_key = 'momentum-demo'),
 (SELECT id FROM teams WHERE team_key = 'DES'),
 (SELECT id FROM roadmaps WHERE name = 'Q3 2025'),
 1, 'UI/UX Redesign', 'UI', 'Complete UI/UX overhaul', 'planned');

INSERT INTO cycles (team_id, name, start_date, end_date, status) VALUES
((SELECT id FROM teams WHERE team_key = 'ENG'), 'Cycle 15', '2025-07-14', '2025-07-28', 'active'),
((SELECT id FROM teams WHERE team_key = 'ENG'), 'Cycle 16', '2025-07-29', '2025-08-12', 'planned');

INSERT INTO labels (workspace_id, name, color) VALUES
((SELECT id FROM workspaces WHERE url_key = 'momentum-demo'), 'Bug', '#ff0000'),
((SELECT id FROM workspaces WHERE url_key = 'momentum-demo'), 'Feature', '#00ff00'),
((SELECT id FROM workspaces WHERE url_key = 'momentum-demo'), 'Enhancement', '#0000ff'),
((SELECT id FROM workspaces WHERE url_key = 'momentum-demo'), 'Documentation', '#ffff00');
