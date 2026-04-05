use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use std::collections::HashMap;
use tracing::debug;

use crate::models::{PaginatedResponse, PaperlessDocument, PaperlessLabel};

const PAGE_SIZE: u64 = 100;

pub struct PaperlessClient {
    client: Client,
}

impl PaperlessClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    fn auth_header(api_key: &str) -> String {
        format!("Token {}", api_key)
    }

    /// Fetch a single page of documents.
    async fn fetch_documents_page(
        &self,
        base_url: &str,
        api_key: &str,
        page: u64,
        modified_after: Option<&str>,
    ) -> Result<PaginatedResponse<PaperlessDocument>> {
        let mut url = format!(
            "{}/api/documents/?page={}&page_size={}",
            base_url, page, PAGE_SIZE
        );
        if let Some(ts) = modified_after {
            url.push_str(&format!("&modified__gt={}", ts));
        }

        debug!("Fetching documents page {}: {}", page, url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", Self::auth_header(api_key))
            .send()
            .await
            .context("Failed to connect to paperless-ngx")?;

        let status = response.status();
        if status == 401 || status == 403 {
            return Err(anyhow!(
                "Authentication failed ({}). Check your paperless-ngx API key.",
                status
            ));
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("paperless-ngx API returned HTTP {}: {}", status, body));
        }

        response
            .json()
            .await
            .context("Failed to parse documents response")
    }

    /// Fetch all documents (paginated), optionally filtered by modification date.
    pub async fn fetch_all_documents(
        &self,
        base_url: &str,
        api_key: &str,
        modified_after: Option<&str>,
    ) -> Result<Vec<PaperlessDocument>> {
        let mut all = Vec::new();
        let mut page = 1u64;

        loop {
            let page_data = self
                .fetch_documents_page(base_url, api_key, page, modified_after)
                .await?;
            let fetched = page_data.results.len();
            debug!("Page {}: got {} documents (total {})", page, fetched, page_data.count);
            all.extend(page_data.results);

            if page_data.next.is_none() {
                break;
            }
            page += 1;
        }

        Ok(all)
    }

    /// Fetch a paginated list of labels (tags, correspondents, document types).
    async fn fetch_labels(&self, url: &str, api_key: &str) -> Result<Vec<PaperlessLabel>> {
        let mut all = Vec::new();
        let mut page = 1u64;

        loop {
            let paged_url = format!("{}&page={}&page_size={}", url, page, PAGE_SIZE);
            debug!("Fetching labels page {}: {}", page, paged_url);

            let response = self
                .client
                .get(&paged_url)
                .header("Authorization", Self::auth_header(api_key))
                .send()
                .await
                .context("Failed to fetch label page")?;

            let status = response.status();
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(anyhow!("paperless-ngx API returned HTTP {}: {}", status, body));
            }

            let page_data: PaginatedResponse<PaperlessLabel> =
                response.json().await.context("Failed to parse labels response")?;

            let fetched = page_data.results.len();
            all.extend(page_data.results);

            if page_data.next.is_none() || fetched == 0 {
                break;
            }
            page += 1;
        }

        Ok(all)
    }

    /// Fetch all tags as an id → name map.
    pub async fn fetch_tags(&self, base_url: &str, api_key: &str) -> Result<HashMap<i64, String>> {
        let url = format!("{}/api/tags/?", base_url);
        let labels = self.fetch_labels(&url, api_key).await?;
        Ok(labels.into_iter().map(|l| (l.id, l.name)).collect())
    }

    /// Fetch all correspondents as an id → name map.
    pub async fn fetch_correspondents(
        &self,
        base_url: &str,
        api_key: &str,
    ) -> Result<HashMap<i64, String>> {
        let url = format!("{}/api/correspondents/?", base_url);
        let labels = self.fetch_labels(&url, api_key).await?;
        Ok(labels.into_iter().map(|l| (l.id, l.name)).collect())
    }

    /// Fetch all document types as an id → name map.
    pub async fn fetch_document_types(
        &self,
        base_url: &str,
        api_key: &str,
    ) -> Result<HashMap<i64, String>> {
        let url = format!("{}/api/document_types/?", base_url);
        let labels = self.fetch_labels(&url, api_key).await?;
        Ok(labels.into_iter().map(|l| (l.id, l.name)).collect())
    }

    /// Test connectivity by fetching one document.
    pub async fn test_connection(&self, base_url: &str, api_key: &str) -> Result<()> {
        let url = format!("{}/api/documents/?page=1&page_size=1", base_url);
        let response = self
            .client
            .get(&url)
            .header("Authorization", Self::auth_header(api_key))
            .send()
            .await
            .context("Failed to connect to paperless-ngx")?;

        let status = response.status();
        if status == 401 || status == 403 {
            return Err(anyhow!(
                "Authentication failed ({}). Check your paperless-ngx URL and API key.",
                status
            ));
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("paperless-ngx returned HTTP {}: {}", status, body));
        }

        Ok(())
    }
}
