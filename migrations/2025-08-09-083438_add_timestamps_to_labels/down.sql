-- Drop trigger on labels table
DROP TRIGGER IF EXISTS update_labels_updated_at ON labels;

-- Remove timestamp columns from labels table
ALTER TABLE labels 
DROP COLUMN created_at,
DROP COLUMN updated_at;