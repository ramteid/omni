-- Create content_blobs table for TOAST-backed content storage
-- Leverages PostgreSQL's TOAST mechanism for automatic large value handling

CREATE TABLE IF NOT EXISTS content_blobs (
    id CHAR(26) PRIMARY KEY,                    -- ULID for content identification
    content BYTEA NOT NULL,                     -- TOAST will handle large values automatically
    content_type VARCHAR(100),                  -- MIME type for metadata (optional)
    size_bytes BIGINT NOT NULL,                 -- Original content size in bytes
    sha256_hash CHAR(64),                       -- SHA256 hash for deduplication (optional)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for efficient lookups by hash (for potential deduplication)
CREATE INDEX IF NOT EXISTS idx_content_blobs_sha256_hash ON content_blobs(sha256_hash);

-- Index for cleanup operations by creation time
CREATE INDEX IF NOT EXISTS idx_content_blobs_created_at ON content_blobs(created_at);

-- Trigger to update updated_at timestamp
CREATE TRIGGER update_content_blobs_updated_at 
    BEFORE UPDATE ON content_blobs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TABLE IF NOT EXISTS documents (
    id CHAR(26) PRIMARY KEY,
    source_id CHAR(26) NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    external_id VARCHAR(500) NOT NULL,
    title TEXT NOT NULL,
    content_id CHAR(26) REFERENCES content_blobs(id) ON DELETE SET NULL,
    content_type VARCHAR(100),
    file_size BIGINT,
    file_extension VARCHAR(50),
    url TEXT,
    metadata JSONB NOT NULL DEFAULT '{}',
    permissions JSONB NOT NULL DEFAULT '[]',
    tsv_content tsvector,  -- Will be populated by indexer from content_blobs content
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_indexed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(source_id, external_id)
);

CREATE INDEX IF NOT EXISTS idx_documents_source_id ON documents(source_id);
CREATE INDEX IF NOT EXISTS idx_documents_external_id ON documents(external_id);
CREATE INDEX IF NOT EXISTS idx_documents_content_id ON documents(content_id);
CREATE INDEX IF NOT EXISTS idx_documents_content_type ON documents(content_type);
CREATE INDEX IF NOT EXISTS idx_documents_tsv_content ON documents USING GIN(tsv_content);
CREATE INDEX IF NOT EXISTS idx_documents_permissions ON documents USING GIN(permissions);
CREATE INDEX IF NOT EXISTS idx_documents_created_at ON documents(created_at);
CREATE INDEX IF NOT EXISTS idx_documents_updated_at ON documents(updated_at);

DROP TRIGGER IF EXISTS update_documents_updated_at ON documents;
CREATE TRIGGER update_documents_updated_at BEFORE UPDATE ON documents
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();