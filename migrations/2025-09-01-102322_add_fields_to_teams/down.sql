-- Remove new fields from teams table
ALTER TABLE teams 
DROP COLUMN IF EXISTS description,
DROP COLUMN IF EXISTS icon_url,
DROP COLUMN IF EXISTS is_private;