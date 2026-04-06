use anyhow::{anyhow, Context, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A Paperless-ngx API client using token authentication.
pub struct PaperlessClient {
    client: Client,
    base_url: String,
}

impl PaperlessClient {
    /// Create a new client.  `base_url` must not have a trailing slash.
    pub fn new(base_url: &str, api_key: &str) -> Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        let auth_value = reqwest::header::HeaderValue::from_str(&format!("Token {}", api_key))
            .context("Invalid API key format")?;
        headers.insert(reqwest::header::AUTHORIZATION, auth_value);

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    /// Validate the API key by fetching the user info endpoint.
    pub async fn validate(&self) -> Result<()> {
        let url = format!("{}/api/ui_settings/", self.base_url);
        let resp = self.client.get(&url).send().await.context("HTTP request failed")?;
        match resp.status() {
            StatusCode::OK => Ok(()),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(anyhow!("Invalid API key"))
            }
            status => Err(anyhow!("Unexpected status {}", status)),
        }
    }

    /// Fetch all correspondents, returning a map of id → display name.
    pub async fn fetch_correspondents(&self) -> Result<HashMap<i64, String>> {
        self.fetch_all_named_resources("correspondents").await
    }

    /// Fetch all document types, returning a map of id → name.
    pub async fn fetch_document_types(&self) -> Result<HashMap<i64, String>> {
        self.fetch_all_named_resources("document_types").await
    }

    /// Fetch all tags, returning a map of id → name.
    pub async fn fetch_tags(&self) -> Result<HashMap<i64, String>> {
        self.fetch_all_named_resources("tags").await
    }

    /// Fetch all storage paths, returning a map of id → path.
    pub async fn fetch_storage_paths(&self) -> Result<HashMap<i64, String>> {
        self.fetch_all_named_resources("storage_paths").await
    }

    async fn fetch_all_named_resources(&self, resource: &str) -> Result<HashMap<i64, String>> {
        let mut map = HashMap::new();
        let mut url = Some(format!("{}/api/{}/", self.base_url, resource));
        while let Some(next_url) = url {
            let resp = self
                .client
                .get(&next_url)
                .send()
                .await
                .with_context(|| format!("Failed to fetch {}", resource))?;
            if !resp.status().is_success() {
                return Err(anyhow!("Failed to fetch {}: HTTP {}", resource, resp.status()));
            }
            let page: PagedResponse<NamedResource> = resp.json().await?;
            for item in page.results {
                map.insert(item.id, item.name);
            }
            url = page.next;
        }
        Ok(map)
    }

    /// Fetch a page of documents.  Returns `(results, next_url)`.
    pub async fn fetch_documents_page(
        &self,
        url: &str,
    ) -> Result<(Vec<PaperlessDocument>, Option<String>)> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch documents page")?;
        if !resp.status().is_success() {
            return Err(anyhow!("Failed to fetch documents: HTTP {}", resp.status()));
        }
        let page: PagedResponse<PaperlessDocument> = resp
            .json()
            .await
            .context("Failed to parse documents response")?;
        Ok((page.results, page.next))
    }

    /// Returns the URL for the first page of documents.
    pub fn documents_first_page_url(&self) -> String {
        format!("{}/api/documents/?page_size=25&ordering=id", self.base_url)
    }

    /// Returns the Paperless-ngx web UI URL for a document.
    pub fn document_url(&self, id: i64) -> String {
        format!("{}/documents/{}/details", self.base_url, id)
    }
}

// ── API response shapes ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PagedResponse<T> {
    pub next: Option<String>,
    pub results: Vec<T>,
}

#[derive(Debug, Deserialize)]
pub struct NamedResource {
    pub id: i64,
    pub name: String,
}

/// Raw document as returned by the Paperless-ngx `/api/documents/` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperlessDocument {
    pub id: i64,
    pub title: String,
    /// OCR'd text content produced by Paperless-ngx.
    #[serde(default)]
    pub content: String,
    pub correspondent: Option<i64>,
    pub document_type: Option<i64>,
    pub storage_path: Option<i64>,
    #[serde(default)]
    pub tags: Vec<i64>,
    /// ISO-8601 creation datetime.
    pub created: Option<String>,
    /// ISO-8601 last-modified datetime (used for change detection).
    pub modified: Option<String>,
    /// ISO-8601 datetime when the document was added to Paperless-ngx.
    pub added: Option<String>,
    pub archive_serial_number: Option<String>,
    pub original_file_name: Option<String>,
    pub archived_file_name: Option<String>,
    #[serde(default)]
    pub notes: Vec<DocumentNote>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentNote {
    pub note: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paperless_document_deserialization() {
        let json = serde_json::json!({
            "id": 42,
            "title": "Invoice 2024",
            "content": "This is the OCR content of the invoice.",
            "correspondent": 5,
            "document_type": 3,
            "storage_path": null,
            "tags": [1, 2],
            "created": "2024-01-15T10:00:00Z",
            "modified": "2024-01-16T10:00:00Z",
            "added": "2024-01-16T10:00:00Z",
            "archive_serial_number": null,
            "original_file_name": "invoice.pdf",
            "archived_file_name": "invoice.pdf",
            "notes": [{"note": "Important document"}]
        });

        let doc: PaperlessDocument = serde_json::from_value(json).unwrap();
        assert_eq!(doc.id, 42);
        assert_eq!(doc.title, "Invoice 2024");
        assert_eq!(doc.content, "This is the OCR content of the invoice.");
        assert_eq!(doc.correspondent, Some(5));
        assert_eq!(doc.document_type, Some(3));
        assert_eq!(doc.tags, vec![1, 2]);
        assert_eq!(doc.notes.len(), 1);
        assert_eq!(doc.notes[0].note, "Important document");
    }

    #[test]
    fn test_paperless_document_minimal_deserialization() {
        let json = serde_json::json!({
            "id": 1,
            "title": "Minimal doc"
        });
        let doc: PaperlessDocument = serde_json::from_value(json).unwrap();
        assert_eq!(doc.id, 1);
        assert_eq!(doc.content, "");
        assert!(doc.tags.is_empty());
        assert!(doc.notes.is_empty());
    }

    #[test]
    fn test_paged_response_deserialization() {
        let json = serde_json::json!({
            "count": 2,
            "next": "http://paperless:8000/api/documents/?page=2",
            "previous": null,
            "results": [
                {"id": 1, "title": "Doc 1"},
                {"id": 2, "title": "Doc 2"}
            ]
        });
        let page: PagedResponse<PaperlessDocument> = serde_json::from_value(json).unwrap();
        assert_eq!(page.results.len(), 2);
        assert_eq!(page.next, Some("http://paperless:8000/api/documents/?page=2".to_string()));
    }
}
