-- 删除索引
DROP INDEX IF EXISTS idx_user_sessions_expires_at;
DROP INDEX IF EXISTS idx_user_sessions_refresh_token;
DROP INDEX IF EXISTS idx_user_sessions_token;
DROP INDEX IF EXISTS idx_user_sessions_user_id;
DROP INDEX IF EXISTS idx_user_credentials_oauth;
DROP INDEX IF EXISTS idx_user_credentials_user_id;

-- 删除表
DROP TABLE IF EXISTS oauth_providers;
DROP TABLE IF EXISTS user_sessions;
DROP TABLE IF EXISTS user_credentials;

-- 恢复用户表
ALTER TABLE users DROP COLUMN IF EXISTS updated_at;
ALTER TABLE users DROP COLUMN IF EXISTS created_at;
ALTER TABLE users DROP COLUMN IF EXISTS is_active;
ALTER TABLE users DROP COLUMN IF EXISTS avatar_url;
ALTER TABLE users DROP COLUMN IF EXISTS username;
ALTER TABLE users DROP COLUMN IF EXISTS email;