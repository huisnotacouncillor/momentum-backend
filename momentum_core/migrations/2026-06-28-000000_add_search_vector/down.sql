DROP TRIGGER IF EXISTS trigger_update_issue_search_vector ON issues;
DROP FUNCTION IF EXISTS update_issue_search_vector();
ALTER TABLE issues DROP COLUMN IF EXISTS search_vector;