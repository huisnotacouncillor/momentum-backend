-- Remove indexes added for login performance optimization
DROP INDEX IF EXISTS idx_users_login_query;
DROP INDEX IF EXISTS idx_user_credentials_login_query;
DROP INDEX IF EXISTS idx_user_credentials_user_id_type_primary;
DROP INDEX IF EXISTS idx_users_is_active;