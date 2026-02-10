ALTER TABLE sources DROP COLUMN sync_status;
ALTER TABLE sources DROP COLUMN sync_error;
ALTER TABLE sources DROP COLUMN last_sync_at;

DROP INDEX IF EXISTS idx_sources_next_sync_at;
ALTER TABLE sources DROP COLUMN next_sync_at;
