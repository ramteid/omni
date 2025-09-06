-- Add user filtering settings to sources table
-- This allows any integration source to configure user-based filtering

-- Add user filtering columns to sources table
ALTER TABLE sources ADD COLUMN user_filter_mode VARCHAR(20) NOT NULL DEFAULT 'all';
ALTER TABLE sources ADD COLUMN user_whitelist JSONB;
ALTER TABLE sources ADD COLUMN user_blacklist JSONB;

-- Add validation constraints
ALTER TABLE sources ADD CONSTRAINT sources_user_filter_mode_check 
    CHECK (user_filter_mode IN ('all', 'whitelist', 'blacklist'));

ALTER TABLE sources ADD CONSTRAINT sources_user_whitelist_check 
    CHECK (user_whitelist IS NULL OR jsonb_typeof(user_whitelist) = 'array');

ALTER TABLE sources ADD CONSTRAINT sources_user_blacklist_check 
    CHECK (user_blacklist IS NULL OR jsonb_typeof(user_blacklist) = 'array');

-- Create indexes for performance
CREATE INDEX idx_sources_user_filter_mode ON sources(user_filter_mode);

-- Add column comments
COMMENT ON COLUMN sources.user_filter_mode IS 'How to filter users: all, whitelist, or blacklist';
COMMENT ON COLUMN sources.user_whitelist IS 'Array of user emails to include when user_filter_mode is whitelist (NULL otherwise)';
COMMENT ON COLUMN sources.user_blacklist IS 'Array of user emails to exclude when user_filter_mode is blacklist (NULL otherwise)';