CREATE TABLE IF NOT EXISTS oauth_credentials (
    id CHAR(26) PRIMARY KEY,
    source_id CHAR(26) NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL,
    client_id VARCHAR(255),
    client_secret TEXT,
    access_token TEXT,
    refresh_token TEXT,
    token_type VARCHAR(50),
    expires_at TIMESTAMPTZ,
    scopes TEXT[],
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT oauth_credentials_provider_check CHECK (provider IN ('google', 'slack', 'atlassian', 'github', 'microsoft'))
);

CREATE INDEX IF NOT EXISTS idx_oauth_credentials_source_id ON oauth_credentials(source_id);
CREATE INDEX IF NOT EXISTS idx_oauth_credentials_provider ON oauth_credentials(provider);
CREATE INDEX IF NOT EXISTS idx_oauth_credentials_expires_at ON oauth_credentials(expires_at);

DROP TRIGGER IF EXISTS update_oauth_credentials_updated_at ON oauth_credentials;
CREATE TRIGGER update_oauth_credentials_updated_at BEFORE UPDATE ON oauth_credentials
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

COMMENT ON TABLE oauth_credentials IS 'Stores OAuth credentials for various service providers';
COMMENT ON COLUMN oauth_credentials.provider IS 'OAuth provider (google, slack, atlassian, github, microsoft)';
COMMENT ON COLUMN oauth_credentials.scopes IS 'Array of OAuth scopes granted';
COMMENT ON COLUMN oauth_credentials.metadata IS 'Provider-specific additional data (e.g., tenant_id, workspace_id)';

ALTER TABLE sources DROP COLUMN IF EXISTS oauth_credentials;