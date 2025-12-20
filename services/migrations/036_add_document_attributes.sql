-- Add dedicated attributes column for structured, filterable document metadata
-- This separates structured key-value data from textual content for better:
-- 1. Semantic search (embeddings only on text content)
-- 2. Faceted filtering (by status, priority, issue_type, etc.)
-- 3. FTS boosting on structured fields

ALTER TABLE documents ADD COLUMN IF NOT EXISTS attributes JSONB NOT NULL DEFAULT '{}';

-- GIN index for fast JSONB containment queries
CREATE INDEX IF NOT EXISTS idx_documents_attributes ON documents USING GIN(attributes);

-- Recreate ParadeDB BM25 index to include attributes as searchable JSON field
DROP INDEX IF EXISTS document_search_idx;

CREATE INDEX document_search_idx ON documents
USING bm25 (id, source_id, external_id, title, content, content_type, file_size, file_extension, metadata, permissions, attributes, created_at, updated_at, last_indexed_at)
WITH (
    key_field = 'id',
    text_fields = '{
        "source_id": { "fast": true, "tokenizer": { "type": "keyword" } },
        "external_id": { "fast": true, "tokenizer": { "type": "keyword" } },
        "content_type": { "fast": true, "tokenizer": { "type": "keyword" } },
        "title": {
            "tokenizer": {
                "type": "ngram",
                "min_gram": 2,
                "max_gram": 3,
                "prefix_only": false
            }
        },
        "content": {
            "tokenizer": {
                "type": "ngram",
                "min_gram": 2,
                "max_gram": 3,
                "prefix_only": false
            }
        }
    }',
    json_fields = '{
        "metadata": {
            "tokenizer": { "type": "keyword" }
        },
        "permissions": {
            "tokenizer": { "type": "keyword" }
        },
        "attributes": {
            "fast": true,
            "tokenizer": { "type": "keyword" }
        }
    }'
);
