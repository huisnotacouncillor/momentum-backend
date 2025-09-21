-- Add indexes to optimize login performance
-- These indexes are specifically designed for the login query pattern:
-- SELECT u.*, uc.* FROM users u
-- INNER JOIN user_credentials uc ON u.id = uc.user_id
-- WHERE u.email = ? AND u.is_active = true
-- AND uc.credential_type = 'password' AND uc.is_primary = true

-- Index for users table login query
CREATE INDEX IF NOT EXISTS idx_users_login_query
ON users (email, is_active)
WHERE is_active = true;

-- Index for user_credentials table login query
CREATE INDEX IF NOT EXISTS idx_user_credentials_login_query
ON user_credentials (user_id, credential_type, is_primary)
WHERE credential_type = 'password' AND is_primary = true;

-- Composite index for the JOIN condition
CREATE INDEX IF NOT EXISTS idx_user_credentials_user_id_type_primary
ON user_credentials (user_id, credential_type, is_primary);

-- Index for is_active filter on users (if not already covered by unique constraint)
CREATE INDEX IF NOT EXISTS idx_users_is_active
ON users (is_active)
WHERE is_active = true;