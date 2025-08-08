-- Change user table ID from integer to UUID
-- This migration handles the conversion step by step

-- First, drop all foreign key constraints that reference users.id
ALTER TABLE user_credentials DROP CONSTRAINT IF EXISTS user_credentials_user_id_fkey;
ALTER TABLE user_sessions DROP CONSTRAINT IF EXISTS user_sessions_user_id_fkey;
ALTER TABLE team_members DROP CONSTRAINT IF EXISTS team_members_user_id_fkey;
ALTER TABLE projects DROP CONSTRAINT IF EXISTS projects_owner_id_fkey;
ALTER TABLE issues DROP CONSTRAINT IF EXISTS issues_creator_id_fkey;
ALTER TABLE issues DROP CONSTRAINT IF EXISTS issues_assignee_id_fkey;
ALTER TABLE comments DROP CONSTRAINT IF EXISTS comments_author_id_fkey;

-- Create a temporary column for the new UUID
ALTER TABLE users ADD COLUMN id_new UUID DEFAULT uuid_generate_v4();

-- Update the new UUID column with generated UUIDs for existing users
UPDATE users SET id_new = uuid_generate_v4() WHERE id_new IS NULL;

-- Make the new column NOT NULL
ALTER TABLE users ALTER COLUMN id_new SET NOT NULL;

-- Step 1: Convert foreign key columns to TEXT first
ALTER TABLE user_credentials ALTER COLUMN user_id TYPE TEXT USING user_id::text;
ALTER TABLE user_sessions ALTER COLUMN user_id TYPE TEXT USING user_id::text;
ALTER TABLE team_members ALTER COLUMN user_id TYPE TEXT USING user_id::text;
ALTER TABLE projects ALTER COLUMN owner_id TYPE TEXT USING owner_id::text;
ALTER TABLE issues ALTER COLUMN creator_id TYPE TEXT USING creator_id::text;
ALTER TABLE issues ALTER COLUMN assignee_id TYPE TEXT USING assignee_id::text;
ALTER TABLE comments ALTER COLUMN author_id TYPE TEXT USING author_id::text;

-- Step 2: Update the text values to use the new UUID values
UPDATE user_credentials SET user_id = u.id_new::text
FROM users u WHERE user_credentials.user_id = u.id::text;

UPDATE user_sessions SET user_id = u.id_new::text
FROM users u WHERE user_sessions.user_id = u.id::text;

UPDATE team_members SET user_id = u.id_new::text
FROM users u WHERE team_members.user_id = u.id::text;

UPDATE projects SET owner_id = u.id_new::text
FROM users u WHERE projects.owner_id = u.id::text;

UPDATE issues SET creator_id = u.id_new::text
FROM users u WHERE issues.creator_id = u.id::text;

UPDATE issues SET assignee_id = u.id_new::text
FROM users u WHERE issues.assignee_id = u.id::text;

UPDATE comments SET author_id = u.id_new::text
FROM users u WHERE comments.author_id = u.id::text;

-- Step 3: Convert TEXT columns to UUID
ALTER TABLE user_credentials ALTER COLUMN user_id TYPE UUID USING user_id::uuid;
ALTER TABLE user_sessions ALTER COLUMN user_id TYPE UUID USING user_id::uuid;
ALTER TABLE team_members ALTER COLUMN user_id TYPE UUID USING user_id::uuid;
ALTER TABLE projects ALTER COLUMN owner_id TYPE UUID USING owner_id::uuid;
ALTER TABLE issues ALTER COLUMN creator_id TYPE UUID USING creator_id::uuid;
ALTER TABLE issues ALTER COLUMN assignee_id TYPE UUID USING assignee_id::uuid;
ALTER TABLE comments ALTER COLUMN author_id TYPE UUID USING author_id::uuid;

-- Drop the old id column and rename id_new to id
ALTER TABLE users DROP COLUMN id;
ALTER TABLE users RENAME COLUMN id_new TO id;
ALTER TABLE users ADD PRIMARY KEY (id);

-- Re-add foreign key constraints
ALTER TABLE user_credentials ADD CONSTRAINT user_credentials_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE user_sessions ADD CONSTRAINT user_sessions_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE team_members ADD CONSTRAINT team_members_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE projects ADD CONSTRAINT projects_owner_id_fkey FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE RESTRICT;
ALTER TABLE issues ADD CONSTRAINT issues_creator_id_fkey FOREIGN KEY (creator_id) REFERENCES users(id) ON DELETE RESTRICT;
ALTER TABLE issues ADD CONSTRAINT issues_assignee_id_fkey FOREIGN KEY (assignee_id) REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE comments ADD CONSTRAINT comments_author_id_fkey FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE CASCADE;
