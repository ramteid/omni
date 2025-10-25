-- Remove the tsv_content trigger and function
-- We now compute tsv_content in the application layer instead of in the database trigger
-- This allows us to work with content stored in S3 instead of just PostgreSQL

DROP TRIGGER IF EXISTS update_document_tsv_content_trigger ON documents;
DROP FUNCTION IF EXISTS update_document_tsv_content();
