-- Change enum columns to TEXT to work with custom enum types

-- Update projects table
ALTER TABLE projects ALTER COLUMN status TYPE TEXT;
ALTER TABLE projects ALTER COLUMN status SET DEFAULT 'planned';

-- Update cycles table
ALTER TABLE cycles ALTER COLUMN status TYPE TEXT;
ALTER TABLE cycles ALTER COLUMN status SET DEFAULT 'planned';

-- Update issues table
ALTER TABLE issues ALTER COLUMN status TYPE TEXT;
ALTER TABLE issues ALTER COLUMN status SET DEFAULT 'backlog';
ALTER TABLE issues ALTER COLUMN priority TYPE TEXT;
ALTER TABLE issues ALTER COLUMN priority SET DEFAULT 'none';

-- Drop the enum types since we're not using them anymore
DROP TYPE IF EXISTS project_status_enum;
DROP TYPE IF EXISTS cycle_status_enum;
DROP TYPE IF EXISTS issue_status_enum;
DROP TYPE IF EXISTS issue_priority_enum;
