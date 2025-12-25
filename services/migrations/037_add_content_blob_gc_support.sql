-- Add garbage collection support for content blobs

-- Track when blob was marked as orphan
ALTER TABLE content_blobs ADD COLUMN IF NOT EXISTS orphaned_at TIMESTAMPTZ;

-- Index for efficient orphan lookup during GC
CREATE INDEX IF NOT EXISTS idx_content_blobs_orphaned_at
ON content_blobs(orphaned_at) WHERE orphaned_at IS NOT NULL;

-- Index for faster content_id extraction from queue payload during GC
CREATE INDEX IF NOT EXISTS idx_queue_payload_content_id
ON connector_events_queue USING BTREE ((payload->>'content_id'))
WHERE status IN ('pending', 'processing') AND payload->>'content_id' IS NOT NULL;
