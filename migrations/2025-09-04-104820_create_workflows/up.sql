-- Create workflows table
CREATE TABLE workflows (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    is_default BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(team_id, name)
);

-- Create workflow_states table
CREATE TABLE workflow_states (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    workflow_id UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    color VARCHAR(7), -- HEX color code like #FF0000
    category VARCHAR(50) NOT NULL, -- Backlog, Unstarted, Started, Completed, Canceled, Triage
    position INTEGER NOT NULL, -- Order within category
    is_default BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workflow_id, name),
    UNIQUE(workflow_id, category, position)
);

-- Create workflow_transitions table for state transitions
CREATE TABLE workflow_transitions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    workflow_id UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    from_state_id UUID REFERENCES workflow_states(id) ON DELETE CASCADE,
    to_state_id UUID NOT NULL REFERENCES workflow_states(id) ON DELETE CASCADE,
    name VARCHAR(255),
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workflow_id, from_state_id, to_state_id)
);

-- Add workflow_id to issues table
ALTER TABLE issues ADD COLUMN workflow_id UUID REFERENCES workflows(id) ON DELETE SET NULL;

-- Add workflow_state_id to issues table
ALTER TABLE issues ADD COLUMN workflow_state_id UUID REFERENCES workflow_states(id) ON DELETE SET NULL;

-- Create indexes for performance
CREATE INDEX idx_workflows_team_id ON workflows(team_id);
CREATE INDEX idx_workflows_is_default ON workflows(is_default);
CREATE INDEX idx_workflow_states_workflow_id ON workflow_states(workflow_id);
CREATE INDEX idx_workflow_states_category ON workflow_states(category);
CREATE INDEX idx_workflow_states_position ON workflow_states(position);
CREATE INDEX idx_workflow_transitions_workflow_id ON workflow_transitions(workflow_id);
CREATE INDEX idx_workflow_transitions_from_state ON workflow_transitions(from_state_id);
CREATE INDEX idx_workflow_transitions_to_state ON workflow_transitions(to_state_id);
CREATE INDEX idx_issues_workflow_id ON issues(workflow_id);
CREATE INDEX idx_issues_workflow_state_id ON issues(workflow_state_id);

-- Insert default workflow for each team
INSERT INTO workflows (id, name, description, team_id, is_default)
SELECT
    uuid_generate_v4(),
    'Default Workflow',
    'Default workflow for ' || t.name || ' team',
    t.id,
    true
FROM teams t;

-- Insert default workflow states for each workflow
INSERT INTO workflow_states (id, workflow_id, name, description, color, category, position, is_default)
SELECT
    uuid_generate_v4(),
    w.id,
    'Backlog',
    'Issues that are not yet prioritized',
    '#999999',
    'Backlog',
    1,
    true
FROM workflows w;

INSERT INTO workflow_states (id, workflow_id, name, description, color, category, position, is_default)
SELECT
    uuid_generate_v4(),
    w.id,
    'Todo',
    'Issues that are ready to be worked on',
    '#6666FF',
    'Unstarted',
    1,
    false
FROM workflows w;

INSERT INTO workflow_states (id, workflow_id, name, description, color, category, position, is_default)
SELECT
    uuid_generate_v4(),
    w.id,
    'In Progress',
    'Issues currently being worked on',
    '#00AA00',
    'Started',
    1,
    false
FROM workflows w;

INSERT INTO workflow_states (id, workflow_id, name, description, color, category, position, is_default)
SELECT
    uuid_generate_v4(),
    w.id,
    'In Review',
    'Issues ready for review',
    '#FFAA00',
    'Started',
    2,
    false
FROM workflows w;

INSERT INTO workflow_states (id, workflow_id, name, description, color, category, position, is_default)
SELECT
    uuid_generate_v4(),
    w.id,
    'Done',
    'Completed issues',
    '#0000FF',
    'Completed',
    1,
    false
FROM workflows w;

INSERT INTO workflow_states (id, workflow_id, name, description, color, category, position, is_default)
SELECT
    uuid_generate_v4(),
    w.id,
    'Canceled',
    'Canceled or invalid issues',
    '#FF0000',
    'Canceled',
    1,
    false
FROM workflows w;

-- Insert default transitions
INSERT INTO workflow_transitions (id, workflow_id, from_state_id, to_state_id, name, description)
SELECT
    uuid_generate_v4(),
    w.id,
    NULL, -- from any state
    ws.id,
    'Create',
    'Create new issue'
FROM workflows w
JOIN workflow_states ws ON ws.workflow_id = w.id
WHERE ws.category = 'Backlog' AND ws.is_default = true;

-- Add transitions between states
INSERT INTO workflow_transitions (id, workflow_id, from_state_id, to_state_id, name, description)
SELECT
    uuid_generate_v4(),
    w.id,
    from_ws.id,
    to_ws.id,
    'Move to ' || to_ws.name,
    'Move issue from ' || from_ws.name || ' to ' || to_ws.name
FROM workflows w
JOIN workflow_states from_ws ON from_ws.workflow_id = w.id
JOIN workflow_states to_ws ON to_ws.workflow_id = w.id
WHERE from_ws.id != to_ws.id;

-- Update existing issues to use default workflow and state
UPDATE issues
SET workflow_id = w.id,
    workflow_state_id = ws.id
FROM workflows w
JOIN workflow_states ws ON ws.workflow_id = w.id
JOIN teams t ON t.id = w.team_id
WHERE w.is_default = true
  AND ws.is_default = true
  AND issues.team_id = t.id;

-- Create trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_workflows_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_workflows_updated_at
    BEFORE UPDATE ON workflows
    FOR EACH ROW
    EXECUTE FUNCTION update_workflows_updated_at();

CREATE OR REPLACE FUNCTION update_workflow_states_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_workflow_states_updated_at
    BEFORE UPDATE ON workflow_states
    FOR EACH ROW
    EXECUTE FUNCTION update_workflow_states_updated_at();
