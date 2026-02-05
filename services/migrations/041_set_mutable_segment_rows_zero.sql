-- Set mutable_segment_rows = 0 to disable the mutable segment
-- This forces all writes to go directly to immutable segments, improving search performance
-- at the cost of slightly higher write latency

ALTER INDEX document_search_idx SET (mutable_segment_rows = 0);
