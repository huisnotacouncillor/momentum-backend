-- Add new fields to teams table to support Linear-like team features
ALTER TABLE teams 
ADD COLUMN description TEXT,
ADD COLUMN icon_url TEXT,
ADD COLUMN is_private BOOLEAN NOT NULL DEFAULT false;

-- Remove the default value to ensure all new records specify is_private explicitly
ALTER TABLE teams 
ALTER COLUMN is_private DROP DEFAULT;