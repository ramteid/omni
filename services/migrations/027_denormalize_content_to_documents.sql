-- Denormalize content from content_blobs into documents table for ParadeDB BM25 indexing

-- Add text content column for searchable text
ALTER TABLE documents
ADD COLUMN IF NOT EXISTS content TEXT;

-- Drop old tsvector-based search infrastructure
DROP INDEX IF EXISTS idx_documents_tsv_content;
DROP FUNCTION IF EXISTS update_document_tsv_content;
DROP TRIGGER IF EXISTS update_document_tsv_content_trigger ON documents;
ALTER TABLE documents DROP COLUMN IF EXISTS tsv_content;


