-- Revert WorkflowStateCategory case changes
-- Restore original uppercase category values

UPDATE workflow_states
SET category = 'Backlog'
WHERE category = 'backlog';

UPDATE workflow_states
SET category = 'Unstarted'
WHERE category = 'unstarted';

UPDATE workflow_states
SET category = 'Started'
WHERE category = 'started';

UPDATE workflow_states
SET category = 'Completed'
WHERE category = 'completed';

UPDATE workflow_states
SET category = 'Canceled'
WHERE category = 'canceled';

UPDATE workflow_states
SET category = 'Triage'
WHERE category = 'triage';