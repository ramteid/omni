-- Change embedding column from vector(1024) to dimensionless vector
ALTER TABLE embeddings ALTER COLUMN embedding TYPE vector;

-- Add dimensions column (NOT NULL, default 1024 for existing rows — PG 11+ metadata-only op)
ALTER TABLE embeddings ADD COLUMN dimensions SMALLINT NOT NULL DEFAULT 1024;
ALTER TABLE embeddings ALTER COLUMN dimensions DROP DEFAULT;

-- Remove the hardcoded default on model_name
ALTER TABLE embeddings ALTER COLUMN model_name DROP DEFAULT;

-- Drop old HNSW index
DROP INDEX IF EXISTS idx_embeddings_vector;

-- Pre-create partial HNSW indexes for supported dimension sizes
CREATE INDEX idx_embeddings_vector_1024 ON embeddings
    USING hnsw ((embedding::vector(1024)) vector_cosine_ops)
    WITH (m = 32, ef_construction = 200)
    WHERE dimensions = 1024;

CREATE INDEX idx_embeddings_vector_768 ON embeddings
    USING hnsw ((embedding::vector(768)) vector_cosine_ops)
    WITH (m = 32, ef_construction = 200)
    WHERE dimensions = 768;

CREATE INDEX idx_embeddings_vector_512 ON embeddings
    USING hnsw ((embedding::vector(512)) vector_cosine_ops)
    WITH (m = 32, ef_construction = 200)
    WHERE dimensions = 512;
