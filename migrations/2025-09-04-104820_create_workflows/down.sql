-- Drop triggers
DROP TRIGGER IF EXISTS update_workflow_states_updated_at ON workflow_states;
DROP TRIGGER IF EXISTS update_workflows_updated_at ON workflows;

-- Drop functions
DROP FUNCTION IF EXISTS update_workflow_states_updated_at();
DROP FUNCTION IF EXISTS update_workflows_updated_at();

-- Remove workflow columns from issues table
ALTER TABLE issues DROP COLUMN IF EXISTS workflow_state_id;
ALTER TABLE issues DROP COLUMN IF EXISTS workflow_id;

-- Drop indexes
DROP INDEX IF EXISTS idx_issues_workflow_state_id;
DROP INDEX IF EXISTS idx_issues_workflow_id;
DROP INDEX IF EXISTS idx_workflow_transitions_to_state;
DROP INDEX IF EXISTS idx_workflow_transitions_from_state;
DROP INDEX IF EXISTS idx_workflow_transitions_workflow_id;
DROP INDEX IF EXISTS idx_workflow_states_position;
DROP INDEX IF EXISTS idx_workflow_states_category;
DROP INDEX IF EXISTS idx_workflow_states_workflow_id;
DROP INDEX IF EXISTS idx_workflows_is_default;
DROP INDEX IF EXISTS idx_workflows_team_id;

-- Drop tables
DROP TABLE IF EXISTS workflow_transitions;
DROP TABLE IF EXISTS workflow_states;
DROP TABLE IF EXISTS workflows;
