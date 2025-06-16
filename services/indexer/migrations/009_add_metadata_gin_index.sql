-- Add GIN index on documents.metadata for efficient facet queries
CREATE INDEX IF NOT EXISTS idx_documents_metadata_gin ON documents USING GIN(metadata);

-- Add GIN index on documents.permissions for owner facet queries
CREATE INDEX IF NOT EXISTS idx_documents_permissions_gin ON documents USING GIN(permissions);

-- Add index on updated_at for time-based facet queries (if not already exists)
CREATE INDEX IF NOT EXISTS idx_documents_updated_at_desc ON documents(updated_at DESC);

-- Add comment explaining the purpose
COMMENT ON INDEX idx_documents_metadata_gin IS 'GIN index for efficient JSONB queries on document metadata for faceting';
COMMENT ON INDEX idx_documents_permissions_gin IS 'GIN index for efficient JSONB queries on document permissions for owner faceting';