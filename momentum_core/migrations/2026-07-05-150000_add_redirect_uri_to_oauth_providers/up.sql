-- Add redirect_uri column to oauth_providers table
ALTER TABLE oauth_providers ADD COLUMN redirect_uri VARCHAR(255);
