-- Add processing_started_at columns to both queue tables for stale processing detection
-- This enables automatic recovery of items stuck in "processing" state after service restarts

-- Add processing_started_at to connector_events_queue
ALTER TABLE connector_events_queue 
ADD COLUMN IF NOT EXISTS processing_started_at TIMESTAMPTZ;

-- Add processing_started_at to embedding_queue
ALTER TABLE embedding_queue 
ADD COLUMN IF NOT EXISTS processing_started_at TIMESTAMPTZ;

-- Create index for efficient stale processing detection on connector_events_queue
CREATE INDEX IF NOT EXISTS idx_connector_events_queue_processing_stale 
ON connector_events_queue(status, processing_started_at) 
WHERE status = 'processing';

-- Create index for efficient stale processing detection on embedding_queue
CREATE INDEX IF NOT EXISTS idx_embedding_queue_processing_stale 
ON embedding_queue(status, processing_started_at) 
WHERE status = 'processing';