-- Remove indexes
DROP INDEX IF EXISTS idx_cycles_end_date;
DROP INDEX IF EXISTS idx_cycles_start_date;
DROP INDEX IF EXISTS idx_cycles_status;

-- Remove columns from cycles table
ALTER TABLE cycles DROP COLUMN IF EXISTS updated_at;
ALTER TABLE cycles DROP COLUMN IF EXISTS goal;
ALTER TABLE cycles DROP COLUMN IF EXISTS description;