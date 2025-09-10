-- Fix WorkflowStateCategory case consistency for existing data
-- This script updates all workflow_states category values to lowercase
-- to match the enum implementation in Rust code

-- Check current data before fixing
SELECT 'Before fix - Current category values:' as status;
SELECT category, COUNT(*) as count 
FROM workflow_states 
GROUP BY category 
ORDER BY category;

-- Update all category values to lowercase
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

-- Verify the changes
SELECT 'After fix - Updated category values:' as status;
SELECT category, COUNT(*) as count 
FROM workflow_states 
GROUP BY category 
ORDER BY category;

-- Show affected records
SELECT 'Affected workflow states:' as status;
SELECT 
    ws.id,
    ws.name,
    ws.category,
    w.name as workflow_name,
    t.name as team_name
FROM workflow_states ws
JOIN workflows w ON ws.workflow_id = w.id
JOIN teams t ON w.team_id = t.id
ORDER BY t.name, w.name, ws.category, ws.position;

-- Summary
SELECT 'Fix completed successfully!' as status;
