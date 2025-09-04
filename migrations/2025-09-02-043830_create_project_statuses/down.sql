-- Add back the status column
ALTER TABLE projects ADD COLUMN status TEXT;

-- Set status values based on project_statuses
UPDATE projects 
SET status = CASE 
    WHEN ps.name = 'Backlog' THEN 'backlog'
    WHEN ps.name = 'Planned' THEN 'planned'
    WHEN ps.name = 'In Progress' THEN 'active'
    WHEN ps.name = 'Completed' THEN 'completed'
    WHEN ps.name = 'Canceled' THEN 'canceled'
    ELSE 'backlog'
END
FROM project_statuses ps
WHERE projects.project_status_id = ps.id;

-- Make status NOT NULL
ALTER TABLE projects ALTER COLUMN status SET NOT NULL;

-- Remove project_status_id column from projects table
ALTER TABLE projects DROP COLUMN project_status_id;

-- Drop project_statuses table
DROP TABLE project_statuses;