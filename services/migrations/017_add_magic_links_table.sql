-- Add magic_links table for passwordless authentication
CREATE TABLE magic_links (
    id CHAR(26) PRIMARY KEY,
    email VARCHAR(255) NOT NULL,
    token_hash VARCHAR(255) NOT NULL UNIQUE,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    used_at TIMESTAMP WITH TIME ZONE NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    user_id CHAR(26) NULL REFERENCES users(id)
);

-- Indexes for fast lookups
CREATE INDEX idx_magic_links_token_hash ON magic_links(token_hash);
CREATE INDEX idx_magic_links_email ON magic_links(email);
CREATE INDEX idx_magic_links_expires_at ON magic_links(expires_at);
CREATE INDEX idx_magic_links_user_id ON magic_links(user_id);

-- Add cleanup function to remove expired magic links
CREATE OR REPLACE FUNCTION cleanup_expired_magic_links()
RETURNS void AS $$
BEGIN
    DELETE FROM magic_links WHERE expires_at < CURRENT_TIMESTAMP;
END;
$$ LANGUAGE plpgsql;

-- Optional: Add function to be called periodically
-- Can be set up as a cron job or called from application
COMMENT ON FUNCTION cleanup_expired_magic_links() IS 'Removes expired magic links from the database';