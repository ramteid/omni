-- Configure background segment merging for ParadeDB BM25 index
-- Added 10MB tier between 1MB and 100MB for smoother compaction
-- This prevents segment accumulation during high-write workloads
ALTER INDEX document_search_idx SET (background_layer_sizes = '100KB, 1MB, 10MB, 100MB, 1GB, 10GB');
