use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub search_mode: String,
    pub limit: i64,
    pub offset: i64,
    pub sources: Option<Vec<String>>,
    pub content_types: Option<Vec<String>>,
    pub include_facets: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total_count: i64,
    pub query_time_ms: u64,
    pub has_more: bool,
    pub query: String,
    pub corrected_query: Option<String>,
    pub corrections: Option<Vec<String>>,
    pub facets: Vec<Facet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub document: Document,
    pub score: f32,
    pub highlights: Vec<String>,
    pub match_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub content: Option<String>,
    pub source: String,
    pub content_type: String,
    pub url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub indexed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Facet {
    pub name: String,
    pub values: Vec<FacetValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetValue {
    pub value: String,
    pub count: i64,
}

pub struct ClioSearchClient {
    client: Client,
    base_url: String,
}

impl ClioSearchClient {
    pub fn new(base_url: &str) -> Result<Self> {
        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    pub async fn search(&self, request: &SearchRequest) -> Result<SearchResponse> {
        let url = format!("{}/search", self.base_url);

        debug!("Sending search request to: {}", url);
        debug!("Request: {:?}", request);

        let response = self.client.post(&url).json(request).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!(
                "Search request failed with status {}: {}",
                status, error_text
            );
            return Err(anyhow::anyhow!(
                "Search request failed with status {}: {}",
                status,
                error_text
            ));
        }

        let search_response: SearchResponse = response.json().await?;
        debug!("Received {} results", search_response.results.len());

        Ok(search_response)
    }

    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);

        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    pub async fn get_index_stats(&self) -> Result<IndexStats> {
        let url = format!("{}/stats", self.base_url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get index stats"));
        }

        let stats: IndexStats = response.json().await?;
        Ok(stats)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_documents: i64,
    pub total_sources: i64,
    pub index_size_mb: f64,
    pub last_updated: String,
}

impl SearchRequest {
    pub fn new(query: String, search_mode: String) -> Self {
        Self {
            query,
            search_mode,
            limit: 20,
            offset: 0,
            sources: None,
            content_types: None,
            include_facets: false,
        }
    }

    pub fn with_limit(mut self, limit: i64) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_offset(mut self, offset: i64) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_sources(mut self, sources: Vec<String>) -> Self {
        self.sources = Some(sources);
        self
    }

    pub fn with_content_types(mut self, content_types: Vec<String>) -> Self {
        self.content_types = Some(content_types);
        self
    }

    pub fn with_facets(mut self, include_facets: bool) -> Self {
        self.include_facets = include_facets;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_request_builder() {
        let request = SearchRequest::new("test query".to_string(), "hybrid".to_string())
            .with_limit(10)
            .with_offset(0)
            .with_sources(vec!["google".to_string()])
            .with_facets(true);

        assert_eq!(request.query, "test query");
        assert_eq!(request.search_mode, "hybrid");
        assert_eq!(request.limit, 10);
        assert_eq!(request.offset, 0);
        assert_eq!(request.sources, Some(vec!["google".to_string()]));
        assert_eq!(request.include_facets, true);
    }
}
