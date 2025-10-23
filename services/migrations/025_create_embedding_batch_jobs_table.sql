-- Create embedding_batch_jobs table for tracking batch inference jobs (cloud-agnostic)
CREATE TABLE IF NOT EXISTS embedding_batch_jobs (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL CHECK (status IN ('pending', 'preparing', 'submitted', 'processing', 'completed', 'failed')),
    provider TEXT NOT NULL, -- 'bedrock', 'vertex', etc.
    external_job_id TEXT, -- Provider-specific job ID (e.g., Bedrock job ARN, Vertex job name)
    input_storage_path TEXT, -- Cloud-agnostic path (e.g., s3://bucket/path or gs://bucket/path)
    output_storage_path TEXT, -- Cloud-agnostic path
    document_count INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    submitted_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    error_message TEXT
);

-- Create indexes for efficient querying
CREATE INDEX idx_embedding_batch_jobs_status ON embedding_batch_jobs(status);
CREATE INDEX idx_embedding_batch_jobs_created ON embedding_batch_jobs(created_at);
CREATE INDEX idx_embedding_batch_jobs_external_job_id ON embedding_batch_jobs(external_job_id) WHERE external_job_id IS NOT NULL;
CREATE INDEX idx_embedding_batch_jobs_provider_status ON embedding_batch_jobs(provider, status);

-- Add batch_job_id column to embedding_queue to link queue items to batch jobs
ALTER TABLE embedding_queue ADD COLUMN IF NOT EXISTS batch_job_id TEXT REFERENCES embedding_batch_jobs(id);

-- Create index for efficient batch job lookups
CREATE INDEX idx_embedding_queue_batch_job ON embedding_queue(batch_job_id) WHERE batch_job_id IS NOT NULL;

-- Create index for finding pending items without batch job assignment
CREATE INDEX idx_embedding_queue_pending_no_batch ON embedding_queue(status, created_at)
    WHERE status='pending' AND batch_job_id IS NULL;
