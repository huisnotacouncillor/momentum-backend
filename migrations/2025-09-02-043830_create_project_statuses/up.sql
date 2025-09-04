CREATE TABLE project_statuses (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    color VARCHAR(7), -- HEX color code like #FF0000
    category VARCHAR(50) NOT NULL, -- Backlog, Planned, In Progress, Completed, Canceled
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert default project statuses
INSERT INTO project_statuses (id, name, description, color, category, workspace_id) 
SELECT 
    uuid_generate_v4(),
    'Backlog',
    'Project is in the backlog and not yet prioritized',
    '#999999',
    'Backlog',
    w.id
FROM workspaces w;

INSERT INTO project_statuses (id, name, description, color, category, workspace_id) 
SELECT 
    uuid_generate_v4(),
    'Planned',
    'Project is planned but not yet started',
    '#6666FF',
    'Planned',
    w.id
FROM workspaces w;

INSERT INTO project_statuses (id, name, description, color, category, workspace_id) 
SELECT 
    uuid_generate_v4(),
    'In Progress',
    'Project is currently being worked on',
    '#00AA00',
    'In Progress',
    w.id
FROM workspaces w;

INSERT INTO project_statuses (id, name, description, color, category, workspace_id) 
SELECT 
    uuid_generate_v4(),
    'Completed',
    'Project has been completed',
    '#0000FF',
    'Completed',
    w.id
FROM workspaces w;

INSERT INTO project_statuses (id, name, description, color, category, workspace_id) 
SELECT 
    uuid_generate_v4(),
    'Canceled',
    'Project has been canceled',
    '#FF0000',
    'Canceled',
    w.id
FROM workspaces w;

-- Add project_status_id column to projects table
ALTER TABLE projects ADD COLUMN project_status_id UUID REFERENCES project_statuses(id);

-- Set default project_status_id based on existing status values
UPDATE projects 
SET project_status_id = ps.id
FROM project_statuses ps
JOIN workspaces w ON ps.workspace_id = w.id
WHERE ps.name = 'Backlog' AND projects.status = 'backlog' AND projects.workspace_id = w.id;

UPDATE projects 
SET project_status_id = ps.id
FROM project_statuses ps
JOIN workspaces w ON ps.workspace_id = w.id
WHERE ps.name = 'Planned' AND projects.status = 'planned' AND projects.workspace_id = w.id;

UPDATE projects 
SET project_status_id = ps.id
FROM project_statuses ps
JOIN workspaces w ON ps.workspace_id = w.id
WHERE ps.name = 'In Progress' AND projects.status = 'active' AND projects.workspace_id = w.id;

UPDATE projects 
SET project_status_id = ps.id
FROM project_statuses ps
JOIN workspaces w ON ps.workspace_id = w.id
WHERE ps.name = 'In Progress' AND projects.status = 'paused' AND projects.workspace_id = w.id;

UPDATE projects 
SET project_status_id = ps.id
FROM project_statuses ps
JOIN workspaces w ON ps.workspace_id = w.id
WHERE ps.name = 'Completed' AND projects.status = 'completed' AND projects.workspace_id = w.id;

UPDATE projects 
SET project_status_id = ps.id
FROM project_statuses ps
JOIN workspaces w ON ps.workspace_id = w.id
WHERE ps.name = 'Canceled' AND projects.status = 'canceled' AND projects.workspace_id = w.id;

-- Make project_status_id NOT NULL
ALTER TABLE projects ALTER COLUMN project_status_id SET NOT NULL;

-- Remove old status column
ALTER TABLE projects DROP COLUMN status;