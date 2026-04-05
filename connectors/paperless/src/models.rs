use serde::{Deserialize, Serialize};
use serde_json::json;
use shared::models::{ConnectorEvent, DocumentMetadata, DocumentPermissions};
use std::collections::HashMap;
use time::OffsetDateTime;

/// Persistent sync state stored in `Source.connector_state`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PaperlessConnectorState {
    /// RFC 3339 timestamp of the last successful sync completion.
    /// Used to filter documents on incremental sync (`modified__gt`).
    pub last_sync_at: Option<String>,
    /// IDs of all documents indexed during the last full sync.
    /// Used for deletion detection on the next full sync.
    #[serde(default)]
    pub indexed_ids: Vec<i64>,
}

impl PaperlessConnectorState {
    pub fn from_connector_state(state: &Option<serde_json::Value>) -> Self {
        state
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| json!({}))
    }
}

/// A single document returned by the paperless-ngx `/api/documents/` endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct PaperlessDocument {
    pub id: i64,
    pub title: String,
    /// OCR-extracted text content; may be empty if OCR is disabled.
    #[serde(default)]
    pub content: String,
    /// IDs of tags assigned to this document.
    #[serde(default)]
    pub tags: Vec<i64>,
    /// ID of the correspondent, if any.
    pub correspondent: Option<i64>,
    /// ID of the document type, if any.
    pub document_type: Option<i64>,
    /// ID of the storage path, if any.
    pub storage_path: Option<i64>,
    /// User-set document creation date (RFC 3339).
    pub created: Option<String>,
    /// Last modification date (RFC 3339).
    pub modified: Option<String>,
    /// Date the document was added to paperless-ngx (RFC 3339).
    pub added: Option<String>,
    /// Original filename uploaded by the user.
    pub original_file_name: Option<String>,
    /// Filename of the archived (post-OCR) version.
    pub archived_file_name: Option<String>,
    /// Username of the document owner.
    pub owner: Option<String>,
}

impl PaperlessDocument {
    /// Build a stable external document ID.
    pub fn external_id(&self, source_id: &str) -> String {
        format!("paperless:{}:{}", source_id, self.id)
    }

    /// Build the document URL for linking back to paperless-ngx.
    pub fn document_url(&self, base_url: &str) -> String {
        format!("{}/documents/{}/details", base_url, self.id)
    }

    /// Generate markdown content combining metadata and OCR text.
    pub fn generate_content(
        &self,
        base_url: &str,
        tag_names: &HashMap<i64, String>,
        correspondent_names: &HashMap<i64, String>,
        document_type_names: &HashMap<i64, String>,
    ) -> String {
        let mut out = String::new();

        out.push_str(&format!("# {}\n\n", self.title));

        // Metadata section
        if let Some(created) = &self.created {
            out.push_str(&format!("**Date Created:** {}\n", created));
        }
        if let Some(modified) = &self.modified {
            out.push_str(&format!("**Modified:** {}\n", modified));
        }
        if let Some(added) = &self.added {
            out.push_str(&format!("**Added:** {}\n", added));
        }

        if let Some(corr_id) = self.correspondent {
            if let Some(name) = correspondent_names.get(&corr_id) {
                out.push_str(&format!("**Correspondent:** {}\n", name));
            }
        }

        if let Some(dt_id) = self.document_type {
            if let Some(name) = document_type_names.get(&dt_id) {
                out.push_str(&format!("**Document Type:** {}\n", name));
            }
        }

        if !self.tags.is_empty() {
            let tag_list: Vec<&str> = self
                .tags
                .iter()
                .filter_map(|id| tag_names.get(id).map(|s| s.as_str()))
                .collect();
            if !tag_list.is_empty() {
                out.push_str(&format!("**Tags:** {}\n", tag_list.join(", ")));
            }
        }

        if let Some(filename) = &self.original_file_name {
            out.push_str(&format!("**Original File:** {}\n", filename));
        }

        if let Some(owner) = &self.owner {
            out.push_str(&format!("**Owner:** {}\n", owner));
        }

        out.push_str(&format!("**URL:** {}\n", self.document_url(base_url)));

        // Content section
        let content = self.content.trim();
        if !content.is_empty() {
            out.push_str("\n---\n\n");
            out.push_str(content);
            out.push('\n');
        }

        out
    }

    /// Build a `ConnectorEvent::DocumentCreated` for this document.
    pub fn to_connector_event(
        &self,
        sync_run_id: String,
        source_id: String,
        content_id: String,
        base_url: &str,
        tag_names: &HashMap<i64, String>,
        correspondent_names: &HashMap<i64, String>,
        document_type_names: &HashMap<i64, String>,
    ) -> ConnectorEvent {
        let created_at = self
            .created
            .as_deref()
            .and_then(|s| OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok());
        let updated_at = self
            .modified
            .as_deref()
            .and_then(|s| OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok());

        let tag_list: Vec<String> = self
            .tags
            .iter()
            .filter_map(|id| tag_names.get(id).cloned())
            .collect();

        let mut extra: HashMap<String, serde_json::Value> = HashMap::new();
        let mut paperless_extra: HashMap<String, serde_json::Value> = HashMap::new();
        if let Some(corr_id) = self.correspondent {
            if let Some(name) = correspondent_names.get(&corr_id) {
                paperless_extra.insert("correspondent".to_string(), json!(name));
            }
        }
        if let Some(dt_id) = self.document_type {
            if let Some(name) = document_type_names.get(&dt_id) {
                paperless_extra.insert("document_type".to_string(), json!(name));
            }
        }
        if !tag_list.is_empty() {
            paperless_extra.insert("tags".to_string(), json!(tag_list));
        }
        if let Some(filename) = &self.original_file_name {
            paperless_extra.insert("original_file_name".to_string(), json!(filename));
        }
        if let Some(owner) = &self.owner {
            paperless_extra.insert("owner".to_string(), json!(owner));
        }
        extra.insert("paperless".to_string(), json!(paperless_extra));

        let metadata = DocumentMetadata {
            title: Some(self.title.clone()),
            author: self.owner.clone(),
            created_at,
            updated_at,
            content_type: Some("document".to_string()),
            mime_type: Some("text/markdown".to_string()),
            size: None,
            url: Some(self.document_url(base_url)),
            path: None,
            extra: Some(extra),
        };

        let permissions = DocumentPermissions {
            public: true,
            users: vec![],
            groups: vec![],
        };

        ConnectorEvent::DocumentCreated {
            sync_run_id,
            document_id: self.external_id(&source_id),
            source_id,
            content_id,
            metadata,
            permissions,
            attributes: None,
        }
    }
}

/// A simple label object (tag, correspondent, document type, storage path).
#[derive(Debug, Clone, Deserialize)]
pub struct PaperlessLabel {
    pub id: i64,
    pub name: String,
}

/// Paginated API response from paperless-ngx.
#[derive(Debug, Deserialize)]
pub struct PaginatedResponse<T> {
    pub count: i64,
    pub next: Option<String>,
    pub results: Vec<T>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_doc(id: i64, title: &str, content: &str) -> PaperlessDocument {
        PaperlessDocument {
            id,
            title: title.to_string(),
            content: content.to_string(),
            tags: vec![],
            correspondent: None,
            document_type: None,
            storage_path: None,
            created: Some("2024-01-15T10:00:00Z".to_string()),
            modified: Some("2024-01-16T12:00:00Z".to_string()),
            added: Some("2024-01-16T12:00:00Z".to_string()),
            original_file_name: Some("invoice.pdf".to_string()),
            archived_file_name: None,
            owner: Some("admin".to_string()),
        }
    }

    #[test]
    fn test_external_id_format() {
        let doc = make_doc(42, "Invoice", "content");
        assert_eq!(doc.external_id("src123"), "paperless:src123:42");
    }

    #[test]
    fn test_document_url_format() {
        let doc = make_doc(5, "Test", "");
        assert_eq!(
            doc.document_url("http://localhost:8000"),
            "http://localhost:8000/documents/5/details"
        );
    }

    #[test]
    fn test_generate_content_includes_metadata() {
        let doc = make_doc(1, "My Invoice", "Invoice text here.");
        let tags = HashMap::new();
        let correspondents = HashMap::new();
        let doc_types = HashMap::new();
        let content = doc.generate_content("http://localhost:8000", &tags, &correspondents, &doc_types);

        assert!(content.contains("# My Invoice"));
        assert!(content.contains("**Date Created:** 2024-01-15T10:00:00Z"));
        assert!(content.contains("**Modified:** 2024-01-16T12:00:00Z"));
        assert!(content.contains("**Original File:** invoice.pdf"));
        assert!(content.contains("**Owner:** admin"));
        assert!(content.contains("Invoice text here."));
    }

    #[test]
    fn test_generate_content_with_tags_and_correspondent() {
        let mut doc = make_doc(2, "Contract", "Contract text.");
        doc.tags = vec![10, 20];
        doc.correspondent = Some(5);
        doc.document_type = Some(3);

        let mut tags = HashMap::new();
        tags.insert(10i64, "legal".to_string());
        tags.insert(20i64, "2024".to_string());

        let mut correspondents = HashMap::new();
        correspondents.insert(5i64, "ACME Corp".to_string());

        let mut doc_types = HashMap::new();
        doc_types.insert(3i64, "Contract".to_string());

        let content = doc.generate_content("http://localhost:8000", &tags, &correspondents, &doc_types);

        assert!(content.contains("**Tags:** "));
        assert!(content.contains("legal"));
        assert!(content.contains("2024"));
        assert!(content.contains("**Correspondent:** ACME Corp"));
        assert!(content.contains("**Document Type:** Contract"));
    }

    #[test]
    fn test_generate_content_empty_content_no_separator() {
        let doc = make_doc(3, "Empty Doc", "");
        let content = doc.generate_content(
            "http://localhost:8000",
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
        );
        // No separator when there's no text content
        assert!(!content.contains("---"));
    }

    #[test]
    fn test_connector_state_roundtrip() {
        let state = PaperlessConnectorState {
            last_sync_at: Some("2024-01-15T10:00:00Z".to_string()),
            indexed_ids: vec![1, 2, 3],
        };

        let json = state.to_json();
        let restored = PaperlessConnectorState::from_connector_state(&Some(json));
        assert_eq!(restored.last_sync_at, state.last_sync_at);
        assert_eq!(restored.indexed_ids, state.indexed_ids);
    }

    #[test]
    fn test_connector_state_defaults_on_missing() {
        let state = PaperlessConnectorState::from_connector_state(&None);
        assert!(state.last_sync_at.is_none());
        assert!(state.indexed_ids.is_empty());
    }
}
