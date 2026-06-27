-- Remove constraint
ALTER TABLE labels 
DROP COLUMN level;

-- Make color column optional again
ALTER TABLE labels 
ALTER COLUMN color DROP NOT NULL;

-- Drop enum type
DROP TYPE label_level_enum;