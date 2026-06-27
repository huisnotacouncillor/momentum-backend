-- Fix WorkflowStateCategory case consistency
-- Update all workflow_states category values to lowercase to match other enums

UPDATE workflow_states
SET category = 'backlog'
WHERE category = 'Backlog';

UPDATE workflow_states
SET category = 'unstarted'
WHERE category = 'Unstarted';

UPDATE workflow_states
SET category = 'started'
WHERE category = 'Started';

UPDATE workflow_states
SET category = 'completed'
WHERE category = 'Completed';

UPDATE workflow_states
SET category = 'canceled'
WHERE category = 'Canceled';

UPDATE workflow_states
SET category = 'triage'
WHERE category = 'Triage';