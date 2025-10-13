-- Add storage_backend column to track which backend stores each blob
-- This allows for hybrid deployments where some content is in Postgres and some in S3
--
-- Design: content_blobs becomes a metadata table that tracks all content regardless of backend
-- - id: Always our internal ULID (unchanged)
-- - content: BYTEA, NULL for external storage, populated for Postgres
-- - storage_key: VARCHAR, the key/path in external storage (S3, GCS, etc), NULL for Postgres
-- - storage_backend: Indicates which backend stores the content
--
-- This approach allows us to:
-- 1. Keep stable internal IDs that don't change when migrating between backends
-- 2. Track metadata (size, hash, content_type) for all storage backends
-- 3. Support hybrid deployments and future backend migrations

-- Make content column nullable (it will be NULL for S3-backed blobs)
ALTER TABLE content_blobs
ALTER COLUMN content DROP NOT NULL;

-- Add storage_key column to store the key/path in external storage (S3, GCS, Azure, etc)
ALTER TABLE content_blobs
ADD COLUMN IF NOT EXISTS storage_key VARCHAR(255);

-- Add storage_backend column
ALTER TABLE content_blobs
ADD COLUMN IF NOT EXISTS storage_backend VARCHAR(20) NOT NULL DEFAULT 'postgres';

-- Add index for efficient queries by storage backend
CREATE INDEX IF NOT EXISTS idx_content_blobs_storage_backend ON content_blobs(storage_backend);

-- Add index for storage key lookups
CREATE INDEX IF NOT EXISTS idx_content_blobs_storage_key ON content_blobs(storage_key)
WHERE storage_key IS NOT NULL;

-- Add check constraint to ensure only valid backend types
ALTER TABLE content_blobs
ADD CONSTRAINT chk_storage_backend
CHECK (storage_backend IN ('postgres', 's3'));

-- Add constraints to ensure backend-specific columns are properly set
ALTER TABLE content_blobs
ADD CONSTRAINT chk_postgres_backend_constraints
CHECK (
    storage_backend != 'postgres' OR
    (content IS NOT NULL AND storage_key IS NULL)
);

ALTER TABLE content_blobs
ADD CONSTRAINT chk_s3_backend_constraints
CHECK (
    storage_backend != 's3' OR
    (storage_key IS NOT NULL AND content IS NULL)
);

-- Update existing records to have 'postgres' as their backend
UPDATE content_blobs
SET storage_backend = 'postgres'
WHERE storage_backend IS NULL OR storage_backend = '';
