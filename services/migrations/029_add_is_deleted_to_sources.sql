-- Add is_deleted flag to sources table to differentiate between
-- temporarily disabled sources (is_active=false) and permanently
-- disconnected sources (is_deleted=true)

ALTER TABLE sources
ADD COLUMN is_deleted BOOLEAN NOT NULL DEFAULT false;

COMMENT ON COLUMN sources.is_deleted IS 'Marks source as permanently deleted/disconnected. Unlike is_active which can be toggled, this is a one-way flag. Users must create a new source to re-enable.';
