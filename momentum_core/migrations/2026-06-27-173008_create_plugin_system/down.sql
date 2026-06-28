-- Rollback plugin system migration
DROP TABLE IF EXISTS outbox;
DROP TABLE IF EXISTS agent_runs;
DROP TABLE IF EXISTS plugin_audit;
DROP TABLE IF EXISTS plugin_storage;
DROP TABLE IF EXISTS plugin_installations;
DROP TABLE IF EXISTS plugins;
DROP TABLE IF EXISTS issue_field_values;
DROP TABLE IF EXISTS issue_field_definitions;
ALTER TABLE issues DROP COLUMN IF EXISTS version;