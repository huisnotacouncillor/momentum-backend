-- Remove redirect_uri column from oauth_providers table
ALTER TABLE oauth_providers DROP COLUMN redirect_uri;
