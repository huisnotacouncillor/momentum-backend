-- Add level column to labels table and make color column required
CREATE TYPE label_level_enum AS ENUM ('project', 'issue');

ALTER TABLE labels 
ADD COLUMN level label_level_enum NOT NULL DEFAULT 'issue';

-- Remove the default value to ensure all new records specify level explicitly
ALTER TABLE labels 
ALTER COLUMN level DROP DEFAULT;

-- Make color column required
UPDATE labels 
SET color = '#000000' 
WHERE color IS NULL;

ALTER TABLE labels 
ALTER COLUMN color SET NOT NULL;