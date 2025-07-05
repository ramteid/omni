use crate::models::DocumentChunk;

pub struct ContentChunker;

impl ContentChunker {
    /// Chunk document content into smaller pieces for embedding generation
    pub fn chunk_content(content: &str, max_chunk_size: usize) -> Vec<DocumentChunk> {
        if content.is_empty() {
            return vec![];
        }

        if content.len() <= max_chunk_size {
            return vec![DocumentChunk {
                text: content.to_string(),
                index: 0,
            }];
        }

        let mut chunks = Vec::new();
        let mut current_pos = 0;
        let mut chunk_index = 0;

        while current_pos < content.len() {
            let end_pos = std::cmp::min(current_pos + max_chunk_size, content.len());

            // Try to break at sentence boundaries for better semantic coherence
            let chunk_text = if end_pos < content.len() {
                // Look for sentence endings within the last 100 characters
                let search_start = std::cmp::max(current_pos, end_pos.saturating_sub(100));
                let search_slice = &content[search_start..end_pos];

                if let Some(sentence_end) = search_slice.rfind('.') {
                    let actual_end = search_start + sentence_end + 1;
                    content[current_pos..actual_end].to_string()
                } else if let Some(paragraph_end) = search_slice.rfind('\n') {
                    let actual_end = search_start + paragraph_end + 1;
                    content[current_pos..actual_end].to_string()
                } else {
                    // Fallback to character boundary
                    content[current_pos..end_pos].to_string()
                }
            } else {
                content[current_pos..end_pos].to_string()
            };

            let chunk_end = current_pos + chunk_text.len();
            chunks.push(DocumentChunk {
                text: chunk_text,
                index: chunk_index,
            });

            current_pos = chunk_end;
            chunk_index += 1;

            // Prevent infinite loops
            if current_pos >= content.len() {
                break;
            }
        }

        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_empty_content() {
        let chunks = ContentChunker::chunk_content("", 1000);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_small_content() {
        let content = "This is a small piece of content.";
        let chunks = ContentChunker::chunk_content(content, 1000);

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, content);
        assert_eq!(chunks[0].index, 0);
    }

    #[test]
    fn test_chunk_large_content() {
        let content =
            "This is the first sentence. This is the second sentence. This is the third sentence.";
        let chunks = ContentChunker::chunk_content(content, 50);

        assert!(chunks.len() > 1);

        // Verify all chunks are within size limit or break at sentence boundaries
        for chunk in &chunks {
            assert!(chunk.text.len() <= 50 || chunk.text.contains('.'));
        }

        // Verify chunks are properly indexed
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, i as i32);
        }
    }

    #[test]
    fn test_chunk_with_sentence_boundaries() {
        let content =
            "Short sentence. This is a much longer sentence that should be broken up properly.";
        let chunks = ContentChunker::chunk_content(content, 30);

        assert!(chunks.len() > 1);

        // First chunk should end at sentence boundary
        assert!(chunks[0].text.ends_with('.'));
    }
}
