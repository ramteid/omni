use crate::models::{SearchMode, SearchRequest, SearchResponse, SearchResult, SuggestionsResponse};
use anyhow::Result;
use redis::{AsyncCommands, Client as RedisClient};
use shared::db::repositories::{DocumentRepository, EmbeddingRepository};
use shared::models::ChunkResult;
use shared::{AIClient, ContentStorage, DatabasePool, SearcherConfig};
use std::time::Instant;
use tracing::{debug, info, warn};

pub struct SearchEngine {
    db_pool: DatabasePool,
    redis_client: RedisClient,
    ai_client: AIClient,
    content_storage: ContentStorage,
    config: SearcherConfig,
}

impl SearchEngine {
    pub fn new(
        db_pool: DatabasePool,
        redis_client: RedisClient,
        ai_client: AIClient,
        config: SearcherConfig,
    ) -> Self {
        let content_storage = ContentStorage::new(db_pool.pool().clone());
        Self {
            db_pool,
            redis_client,
            ai_client,
            content_storage,
            config,
        }
    }

    fn prepare_document_for_response(
        &self,
        mut doc: shared::models::Document,
    ) -> shared::models::Document {
        // Clear content_id from search responses for security and efficiency
        doc.content_id = None;
        doc
    }

    pub async fn search(&self, request: SearchRequest) -> Result<SearchResponse> {
        let start_time = Instant::now();

        info!(
            "Searching for query: '{}', mode: {:?}",
            request.query,
            request.search_mode()
        );

        // Generate cache key based on request parameters
        let cache_key = self.generate_cache_key(&request);

        // Try to get from cache first
        if let Ok(mut conn) = self.redis_client.get_multiplexed_async_connection().await {
            if let Ok(cached_response) = conn.get::<_, String>(&cache_key).await {
                if let Ok(response) = serde_json::from_str::<SearchResponse>(&cached_response) {
                    info!("Cache hit for query: '{}'", request.query);
                    return Ok(response);
                }
            }
        }

        let repo = DocumentRepository::new(self.db_pool.pool());
        let limit = request.limit();

        if request.query.trim().is_empty() {
            return Err(anyhow::anyhow!("Search query cannot be empty"));
        }

        let (results, corrected_query) = match request.search_mode() {
            SearchMode::Fulltext => self.fulltext_search(&repo, &request).await?,
            SearchMode::Semantic => (self.semantic_search(&request).await?, None),
            SearchMode::Hybrid => self.hybrid_search(&request).await?,
        };

        let total_count = results.len() as i64;
        let has_more = results.len() as i64 >= limit;
        let query_time = start_time.elapsed().as_millis() as u64;

        info!(
            "Search completed in {}ms, found {} results",
            query_time,
            results.len()
        );

        // Get facets if requested
        let facets = if request.include_facets() {
            let sources = request.sources.as_deref();
            let content_types = request.content_types.as_deref();

            repo.get_facet_counts_with_filters(&request.query, sources, content_types)
                .await
                .unwrap_or_else(|e| {
                    info!("Failed to get facet counts: {}", e);
                    vec![]
                })
        } else {
            vec![]
        };

        let response = SearchResponse {
            results,
            total_count,
            query_time_ms: query_time,
            has_more,
            query: request.query,
            corrected_query,
            corrections: None, // TODO: implement word-level corrections tracking
            facets: if facets.is_empty() {
                None
            } else {
                Some(facets)
            },
        };

        // Cache the response for 5 minutes
        if let Ok(mut conn) = self.redis_client.get_multiplexed_async_connection().await {
            if let Ok(response_json) = serde_json::to_string(&response) {
                let _: Result<(), _> = conn.set_ex(&cache_key, response_json, 300).await;
            }
        }

        Ok(response)
    }

    async fn fulltext_search(
        &self,
        repo: &DocumentRepository,
        request: &SearchRequest,
    ) -> Result<(Vec<SearchResult>, Option<String>)> {
        let start_time = Instant::now();
        let sources = request.sources.as_deref();
        let content_types = request.content_types.as_deref();

        let (results_with_highlights, corrected_query) = if self.config.typo_tolerance_enabled {
            debug!("Searching for {} with typo tolerance", &request.query);
            repo.search_with_typo_tolerance_and_filters(
                &request.query,
                sources,
                content_types,
                request.limit(),
                request.offset(),
                self.config.typo_tolerance_max_distance,
                self.config.typo_tolerance_min_word_length,
                request.user_email().map(|e| e.as_str()),
            )
            .await?
        } else {
            debug!("Searching for {} without typo tolerance", &request.query);
            (
                repo.search_with_filters(
                    &request.query,
                    sources,
                    content_types,
                    request.limit(),
                    request.offset(),
                    request.user_email().map(|e| e.as_str()),
                )
                .await?,
                None,
            )
        };

        let mut results = Vec::new();
        for result_with_highlight in results_with_highlights {
            let prepared_doc = self.prepare_document_for_response(result_with_highlight.document);

            // Convert the single highlight string into a Vec<String> for the highlights field
            let highlights = if result_with_highlight.highlight.trim().is_empty() {
                vec![]
            } else {
                vec![result_with_highlight.highlight]
            };

            results.push(SearchResult {
                document: prepared_doc,
                score: 1.0,
                highlights,
                match_type: "fulltext".to_string(),
                content: None, // No longer fetching full content since we have highlights
            });
        }

        info!(
            "Fulltext search completed in {}ms",
            start_time.elapsed().as_millis()
        );
        Ok((results, corrected_query))
    }

    async fn semantic_search(&self, request: &SearchRequest) -> Result<Vec<SearchResult>> {
        let start_time = Instant::now();
        info!("Performing semantic search for query: '{}'", request.query);

        let query_embedding = self.generate_query_embedding(&request.query).await?;

        let embedding_repo = EmbeddingRepository::new(self.db_pool.pool());
        let doc_repo = DocumentRepository::new(self.db_pool.pool());

        let sources = request.sources.as_deref();
        let content_types = request.content_types.as_deref();

        let chunk_results = embedding_repo
            .find_similar_with_filters(
                query_embedding,
                sources,
                content_types,
                request.limit(),
                request.offset(),
                request.user_email().map(|e| e.as_str()),
            )
            .await?;

        // Get unique document IDs and batch fetch documents
        let document_ids: Vec<String> = chunk_results
            .iter()
            .map(|chunk| chunk.document_id.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let documents = doc_repo.find_by_ids(&document_ids).await?;
        let documents_map: std::collections::HashMap<String, _> = documents
            .into_iter()
            .map(|doc| (doc.id.clone(), doc))
            .collect();

        // Group chunks by document_id to collect all matching chunks per document
        let mut document_chunks: std::collections::HashMap<String, Vec<&ChunkResult>> =
            std::collections::HashMap::new();
        for chunk_result in &chunk_results {
            document_chunks
                .entry(chunk_result.document_id.clone())
                .or_insert_with(Vec::new)
                .push(chunk_result);
        }

        let mut results = Vec::new();
        for (document_id, chunks) in document_chunks {
            if let Some(doc) = documents_map.get(&document_id) {
                // Use the highest scoring chunk as the document score
                let max_score = chunks
                    .iter()
                    .map(|chunk| chunk.similarity_score)
                    .fold(f32::NEG_INFINITY, f32::max);

                // Fetch document content and extract chunk text using offsets
                let mut chunk_highlights: Vec<(f32, String)> = Vec::new();
                if let Some(content_id) = &doc.content_id {
                    if let Ok(content) = self.content_storage.get_text(content_id).await {
                        for chunk in chunks {
                            let chunk_text = self.extract_chunk_from_content(
                                &content,
                                chunk.chunk_start_offset,
                                chunk.chunk_end_offset,
                            );
                            let trimmed_text = chunk_text.trim();

                            if !trimmed_text.is_empty() {
                                let highlight_text = if trimmed_text.len() > 240 {
                                    format!("{}...", &trimmed_text[..240])
                                } else {
                                    trimmed_text.to_string()
                                };
                                chunk_highlights.push((chunk.similarity_score, highlight_text));
                            }
                        }
                    }
                }

                // Sort by similarity score (highest first)
                chunk_highlights
                    .sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

                // Extract just the snippets in sorted order, limited to top 5
                let all_highlights: Vec<String> = chunk_highlights
                    .into_iter()
                    .take(5)
                    .map(|(_, snippet)| snippet)
                    .collect();

                let prepared_doc = self.prepare_document_for_response(doc.clone());
                results.push(SearchResult {
                    document: prepared_doc,
                    score: max_score,
                    highlights: all_highlights,
                    match_type: "semantic".to_string(),
                    content: None, // Using highlights instead of single content snippet
                });
            }
        }

        // Sort results by score in descending order
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        info!(
            "Semantic search completed in {}ms",
            start_time.elapsed().as_millis()
        );
        Ok(results)
    }

    fn extract_chunk_from_content(
        &self,
        content: &str,
        start_offset: i32,
        end_offset: i32,
    ) -> String {
        let start = start_offset as usize;
        let end = end_offset as usize;

        if start >= content.len() || end > content.len() || start >= end {
            return String::new();
        }

        // Find the nearest character boundary at or after start
        let mut actual_start = start;
        while actual_start < content.len() && !content.is_char_boundary(actual_start) {
            actual_start += 1;
        }

        // Find the nearest character boundary at or before end
        let mut actual_end = end;
        while actual_end > actual_start && !content.is_char_boundary(actual_end) {
            actual_end -= 1;
        }

        if actual_start >= actual_end {
            return String::new();
        }

        content[actual_start..actual_end].to_string()
    }

    async fn generate_query_embedding(&self, query: &str) -> Result<Vec<f32>> {
        let embeddings = self
            .ai_client
            .generate_embeddings(vec![query.to_string()])
            .await?;
        if let Some(first_embedding) = embeddings.first() {
            if let Some(first_chunk) = first_embedding.chunk_embeddings.first() {
                return Ok(first_chunk.clone());
            }
        }
        Err(anyhow::anyhow!("Failed to generate embedding for query"))
    }

    async fn hybrid_search(
        &self,
        request: &SearchRequest,
    ) -> Result<(Vec<SearchResult>, Option<String>)> {
        info!("Performing hybrid search for query: '{}'", request.query);
        let start_time = Instant::now();

        // Get results from both FTS and semantic search in parallel
        let repo = DocumentRepository::new(self.db_pool.pool());
        let (fts_future, semantic_future) = (
            self.fulltext_search(&repo, request),
            self.semantic_search(request),
        );
        let (fts_results, semantic_results) = tokio::join!(fts_future, semantic_future);
        let (fts_results, corrected_query) = fts_results?;
        let semantic_results = semantic_results?;
        info!("Retrieved {} results from FTS", fts_results.len());
        info!(
            "Retrieved {} results from semantic search",
            semantic_results.len()
        );

        // Combine and deduplicate results
        let mut combined_results = std::collections::HashMap::new();

        // Add FTS results with normalized scores
        for result in fts_results {
            let doc_id = result.document.id.clone();
            let normalized_score = self.normalize_fts_score(result.score);
            let prepared_doc = self.prepare_document_for_response(result.document);
            combined_results.insert(
                doc_id,
                SearchResult {
                    document: prepared_doc,
                    score: normalized_score * self.config.hybrid_search_fts_weight,
                    highlights: result.highlights,
                    match_type: "hybrid".to_string(),
                    content: result.content,
                },
            );
        }

        // Add or update with semantic results
        for result in semantic_results {
            let doc_id = result.document.id.clone();

            match combined_results.get_mut(&doc_id) {
                Some(existing) => {
                    // Combine scores for documents found in both searches
                    existing.score += result.score * self.config.hybrid_search_semantic_weight;
                }
                None => {
                    // Add new semantic-only result
                    let prepared_doc = self.prepare_document_for_response(result.document);
                    combined_results.insert(
                        doc_id,
                        SearchResult {
                            document: prepared_doc,
                            score: result.score * self.config.hybrid_search_semantic_weight,
                            highlights: result.highlights,
                            match_type: "hybrid".to_string(),
                            content: result.content,
                        },
                    );
                }
            }
        }

        // Convert to vector and sort by combined score
        let mut final_results: Vec<SearchResult> = combined_results.into_values().collect();
        final_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        if final_results.len() > request.limit() as usize {
            final_results.truncate(request.limit() as usize);
        }

        info!(
            "Hybrid search completed in {}ms",
            start_time.elapsed().as_millis()
        );
        Ok((final_results, corrected_query))
    }

    fn normalize_fts_score(&self, score: f32) -> f32 {
        // Simple normalization - in practice this would be more sophisticated
        // based on the actual FTS scoring algorithm
        score.min(1.0).max(0.0)
    }

    fn generate_cache_key(&self, request: &SearchRequest) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        request.query.hash(&mut hasher);
        request.search_mode().hash(&mut hasher);
        request.limit().hash(&mut hasher);
        request.offset().hash(&mut hasher);

        if let Some(sources) = &request.sources {
            for source in sources {
                source.hash(&mut hasher);
            }
        }

        if let Some(content_types) = &request.content_types {
            for ct in content_types {
                ct.hash(&mut hasher);
            }
        }

        request.include_facets().hash(&mut hasher);

        if let Some(user_email) = &request.user_email {
            user_email.hash(&mut hasher);
        }

        format!("search:{:x}", hasher.finish())
    }

    // Simple highlight extraction for semantic search (not using database ts_headline)
    fn extract_highlights(&self, content: &str, query: &str) -> Vec<String> {
        if content.is_empty() || query.is_empty() {
            return vec![];
        }

        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();
        let terms: Vec<&str> = query_lower.split_whitespace().collect();

        // For semantic search, just look for the first occurrence of any term
        for term in terms {
            if term.len() < 3 {
                continue;
            }

            if let Some(pos) = content_lower.find(term) {
                let start = pos.saturating_sub(30);
                let end = (pos + term.len() + 30).min(content.len());

                // Ensure we're on valid UTF-8 character boundaries
                let safe_start = self.find_char_boundary(content, start, true);
                let safe_end = self.find_char_boundary(content, end, false);

                let mut snippet = String::new();
                if safe_start > 0 {
                    snippet.push_str("...");
                }

                let snippet_text = &content[safe_start..safe_end];

                // Find the actual term position within the safe boundaries
                let term_start = pos.max(safe_start);
                let term_end = (pos + term.len()).min(safe_end);

                if term_start < term_end && term_start >= safe_start && term_end <= safe_end {
                    let highlighted = snippet_text.replace(
                        &content[term_start..term_end],
                        &format!("**{}**", &content[term_start..term_end]),
                    );
                    snippet.push_str(&highlighted);
                } else {
                    snippet.push_str(snippet_text);
                }

                if safe_end < content.len() {
                    snippet.push_str("...");
                }

                return vec![snippet];
            }
        }

        vec![]
    }

    fn extract_context_around_matches(&self, content: &str, query: &str) -> String {
        if content.is_empty() || query.is_empty() {
            return String::new();
        }

        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();
        let terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut contexts = Vec::new();

        for term in terms {
            if term.len() < 3 {
                continue;
            }

            if let Some(pos) = content_lower.find(term) {
                // Extract larger context around the match (200 chars before and after)
                let context_start = pos.saturating_sub(200);
                let context_end = (pos + term.len() + 200).min(content.len());

                // Find sentence boundaries for cleaner context, ensuring UTF-8 char boundaries
                let start = if context_start == 0 {
                    0
                } else {
                    let safe_start = self.find_char_boundary(content, context_start, true);
                    content[..safe_start]
                        .rfind('.')
                        .map(|i| i + 1)
                        .unwrap_or(safe_start)
                };

                let end = if context_end >= content.len() {
                    content.len()
                } else {
                    let safe_end = self.find_char_boundary(content, context_end, false);
                    content[safe_end..]
                        .find('.')
                        .map(|i| safe_end + i + 1)
                        .unwrap_or(safe_end)
                };

                let context_text = content[start..end].trim();
                if !context_text.is_empty() && !contexts.contains(&context_text) {
                    contexts.push(context_text);
                }
            }
        }

        // Join contexts and limit total length
        let combined = contexts.join(" ... ");
        if combined.chars().count() > 1000 {
            let truncated: String = combined.chars().take(1000).collect();
            format!("{}...", truncated)
        } else {
            combined
        }
    }

    pub async fn suggest(&self, query: &str, limit: i64) -> Result<SuggestionsResponse> {
        info!("Getting suggestions for query: '{}'", query);

        if query.trim().is_empty() {
            return Ok(SuggestionsResponse {
                suggestions: vec![],
                query: query.to_string(),
            });
        }

        let suggestions = sqlx::query_scalar::<_, String>(
            r#"
            SELECT DISTINCT title
            FROM documents
            WHERE title ILIKE $1
            ORDER BY title
            LIMIT $2
            "#,
        )
        .bind(format!("%{}%", query))
        .bind(limit)
        .fetch_all(self.db_pool.pool())
        .await?;

        info!("Found {} suggestions", suggestions.len());

        Ok(SuggestionsResponse {
            suggestions,
            query: query.to_string(),
        })
    }

    /// Generate RAG context from search request using chunk-based approach
    pub async fn get_rag_context(&self, request: &SearchRequest) -> Result<Vec<SearchResult>> {
        info!("Generating RAG context for query: '{}'", request.query);

        // Get embedding chunks that match the query semantically
        let query_embedding = self.generate_query_embedding(&request.query).await?;
        let embedding_repo = EmbeddingRepository::new(self.db_pool.pool());

        // Get multiple chunks per document for better context
        let embedding_chunks = embedding_repo
            .find_rag_chunks(
                query_embedding,
                3,   // max 3 chunks per document
                0.7, // similarity threshold
                15,  // max 15 total chunks
                request.user_email().map(|e| e.as_str()),
            )
            .await?;

        // Get full-text search results to extract context around exact matches
        let repo = DocumentRepository::new(self.db_pool.pool());
        let (fts_results, _) = self.fulltext_search(&repo, request).await?;

        // Combine embedding chunks and fulltext context
        let mut combined_results = Vec::new();

        // Add embedding chunks as SearchResults
        for (doc, score, chunk_text) in embedding_chunks {
            let mut doc_with_chunk = self.prepare_document_for_response(doc);
            // Create a new document with the specific chunk for semantic matches
            // Note: We don't set content field as it's not part of the Document model anymore

            combined_results.push(SearchResult {
                document: doc_with_chunk,
                score,
                highlights: vec![],
                match_type: "semantic_chunk".to_string(),
                content: Some(chunk_text),
            });
        }

        // Add context around fulltext matches
        for fts_result in fts_results.into_iter().take(5) {
            // For fulltext matches, we already have highlights generated
            // Use the prepared document and existing highlights
            combined_results.push(SearchResult {
                document: fts_result.document,
                score: fts_result.score,
                highlights: fts_result.highlights,
                match_type: "fulltext_context".to_string(),
                content: fts_result.content,
            });
        }

        // Sort by score and take top results
        combined_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        combined_results.truncate(10);

        info!(
            "Generated RAG context with {} chunks",
            combined_results.len()
        );
        Ok(combined_results)
    }

    /// Helper function to find a safe UTF-8 character boundary
    fn find_char_boundary(&self, content: &str, pos: usize, round_down: bool) -> usize {
        if pos == 0 {
            return 0;
        }
        if pos >= content.len() {
            return content.len();
        }

        if content.is_char_boundary(pos) {
            return pos;
        }

        if round_down {
            // Find the nearest character boundary at or before pos
            let mut safe_pos = pos;
            while safe_pos > 0 && !content.is_char_boundary(safe_pos) {
                safe_pos -= 1;
            }
            safe_pos
        } else {
            // Find the nearest character boundary at or after pos
            let mut safe_pos = pos;
            while safe_pos < content.len() && !content.is_char_boundary(safe_pos) {
                safe_pos += 1;
            }
            safe_pos
        }
    }

    /// Build RAG prompt with context chunks and citation instructions
    pub fn build_rag_prompt(&self, query: &str, context: &[SearchResult]) -> String {
        let mut prompt = String::new();

        prompt.push_str("You are a helpful AI assistant that answers questions based on the provided context from various documents. ");
        prompt.push_str(
            "Please provide a comprehensive answer using the information from the context. ",
        );
        prompt.push_str(
            "When referencing information, cite it using the format [Source: Document Title]. ",
        );
        prompt.push_str(
            "If the context doesn't contain enough information to answer the question, say so.\n\n",
        );

        prompt.push_str("Context Information:\n");
        for (i, result) in context.iter().enumerate() {
            prompt.push_str(&format!(
                "Context {}: From \"{}\" ({})\n",
                i + 1,
                result.document.title,
                result.match_type
            ));

            match result.match_type.as_str() {
                "semantic_chunk" => {
                    // For semantic chunks, use the highlights if available
                    if !result.highlights.is_empty() {
                        prompt.push_str(&format!("Content: {}\n", result.highlights[0]));
                    }
                }
                "fulltext_context" => {
                    // For fulltext matches, use the highlights which contain context around matches
                    if !result.highlights.is_empty() {
                        prompt.push_str(&format!("Relevant excerpt: {}\n", result.highlights[0]));
                    }
                }
                _ => {
                    // Fallback: try to get content from LOB storage for other match types
                    if let Some(_content_id) = &result.document.content_id {
                        // Note: In a real implementation, we would need to fetch from LOB storage
                        // For now, we'll use the highlights if available
                        if !result.highlights.is_empty() {
                            prompt.push_str(&format!("Content: {}\n", result.highlights[0]));
                        }
                    }
                }
            }
            prompt.push_str("\n");
        }

        prompt.push_str(&format!("Question: {}\n\n", query));
        prompt.push_str("Answer:");

        prompt
    }
}
