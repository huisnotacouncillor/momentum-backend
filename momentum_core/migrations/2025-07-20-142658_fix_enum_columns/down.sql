-- Recreate enum types
CREATE TYPE project_status_enum AS ENUM ('planned', 'active', 'paused', 'completed', 'canceled');
CREATE TYPE cycle_status_enum AS ENUM ('planned', 'active', 'completed');
CREATE TYPE issue_status_enum AS ENUM ('backlog', 'todo', 'in_progress', 'in_review', 'done', 'canceled');
CREATE TYPE issue_priority_enum AS ENUM ('none', 'low', 'medium', 'high', 'urgent');

-- Revert projects table
ALTER TABLE projects ALTER COLUMN status TYPE project_status_enum USING status::project_status_enum;
ALTER TABLE projects ALTER COLUMN status SET DEFAULT 'planned'::project_status_enum;

-- Revert cycles table
ALTER TABLE cycles ALTER COLUMN status TYPE cycle_status_enum USING status::cycle_status_enum;
ALTER TABLE cycles ALTER COLUMN status SET DEFAULT 'planned'::cycle_status_enum;

-- Revert issues table
ALTER TABLE issues ALTER COLUMN status TYPE issue_status_enum USING status::issue_status_enum;
ALTER TABLE issues ALTER COLUMN status SET DEFAULT 'backlog'::issue_status_enum;
ALTER TABLE issues ALTER COLUMN priority TYPE issue_priority_enum USING priority::issue_priority_enum;
ALTER TABLE issues ALTER COLUMN priority SET DEFAULT 'none'::issue_priority_enum;
