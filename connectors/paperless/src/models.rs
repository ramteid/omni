use serde::{Deserialize, Serialize};
use serde_json::json;
use shared::models::{
    ConnectorEvent, DocumentMetadata, DocumentPermissions,
};
use std::collections::{HashMap, HashSet};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::client::PaperlessDocument;

/// Persistent sync state stored in `Source.connector_state`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PaperlessConnectorState {
    /// Set of document IDs that have been successfully indexed.
    #[serde(default)]
    pub indexed_ids: HashSet<i64>,
    /// Maps document ID → `modified` timestamp string for change detection.
    #[serde(default)]
    pub modified_at: HashMap<i64, String>,
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

/// All resolved metadata for a document (IDs replaced with display names).
pub struct ResolvedDocument<'a> {
    pub doc: &'a PaperlessDocument,
    pub correspondent_name: Option<&'a str>,
    pub document_type_name: Option<&'a str>,
    pub tag_names: Vec<&'a str>,
    pub storage_path_name: Option<&'a str>,
}

/// Build a stable external document ID for a Paperless-ngx document.
pub fn make_document_id(source_id: &str, doc_id: i64) -> String {
    format!("paperless:{}:{}", source_id, doc_id)
}

/// Generate the indexable content for a document in Markdown format,
/// combining OCR text with comprehensive metadata.
pub fn generate_document_content(resolved: &ResolvedDocument<'_>) -> String {
    let doc = resolved.doc;
    let mut out = String::new();

    // Title as heading
    out.push_str(&format!("# {}\n\n", doc.title));

    // Metadata section
    let mut meta_lines: Vec<String> = Vec::new();
    if let Some(name) = resolved.correspondent_name {
        meta_lines.push(format!("**Correspondent:** {}", name));
    }
    if let Some(name) = resolved.document_type_name {
        meta_lines.push(format!("**Document Type:** {}", name));
    }
    if !resolved.tag_names.is_empty() {
        meta_lines.push(format!("**Tags:** {}", resolved.tag_names.join(", ")));
    }
    if let Some(path) = resolved.storage_path_name {
        meta_lines.push(format!("**Storage Path:** {}", path));
    }
    if let Some(created) = &doc.created {
        meta_lines.push(format!("**Created:** {}", created));
    }
    if let Some(added) = &doc.added {
        meta_lines.push(format!("**Added:** {}", added));
    }
    if let Some(asn) = &doc.archive_serial_number {
        meta_lines.push(format!("**Archive Serial Number:** {}", asn));
    }
    if let Some(file_name) = &doc.original_file_name {
        meta_lines.push(format!("**File:** {}", file_name));
    }

    if !meta_lines.is_empty() {
        for line in &meta_lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }

    // Document content (OCR text)
    if !doc.content.trim().is_empty() {
        out.push_str("---\n\n");
        out.push_str(doc.content.trim());
        out.push('\n');
    }

    // Notes
    if !doc.notes.is_empty() {
        out.push_str("\n---\n\n**Notes:**\n");
        for note in &doc.notes {
            if !note.note.trim().is_empty() {
                out.push_str(&format!("- {}\n", note.note.trim()));
            }
        }
    }

    out
}

/// Parse an ISO-8601 datetime string into an `OffsetDateTime`.
pub fn parse_datetime(s: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339).ok()
}

/// Build a `ConnectorEvent::DocumentCreated` for a Paperless-ngx document.
pub fn build_document_created_event(
    resolved: &ResolvedDocument<'_>,
    sync_run_id: String,
    source_id: String,
    content_id: String,
    doc_url: String,
    user_email: Option<&str>,
) -> ConnectorEvent {
    let doc = resolved.doc;
    let document_id = make_document_id(&source_id, doc.id);

    let mut attributes = serde_json::Map::new();
    if let Some(name) = resolved.correspondent_name {
        attributes.insert("correspondent".to_string(), json!(name));
    }
    if let Some(name) = resolved.document_type_name {
        attributes.insert("document_type".to_string(), json!(name));
    }
    if !resolved.tag_names.is_empty() {
        attributes.insert("tags".to_string(), json!(resolved.tag_names));
    }
    if let Some(asn) = &doc.archive_serial_number {
        attributes.insert("archive_serial_number".to_string(), json!(asn));
    }

    let created_at = doc.created.as_deref().and_then(parse_datetime);
    let updated_at = doc.modified.as_deref().and_then(parse_datetime);

    let metadata = DocumentMetadata {
        title: Some(doc.title.clone()),
        author: resolved.correspondent_name.map(str::to_string),
        created_at,
        updated_at,
        content_type: resolved.document_type_name.map(str::to_string),
        mime_type: None,
        size: None,
        url: Some(doc_url),
        path: None,
        extra: None,
    };

    let permissions = DocumentPermissions {
        public: false,
        users: user_email.map(|e| vec![e.to_string()]).unwrap_or_default(),
        groups: vec![],
    };

    ConnectorEvent::DocumentCreated {
        sync_run_id,
        source_id,
        document_id,
        content_id,
        metadata,
        permissions,
        attributes: if attributes.is_empty() {
            None
        } else {
            Some(attributes.into_iter().collect())
        },
    }
}

/// Build a `ConnectorEvent::DocumentUpdated` for a Paperless-ngx document.
pub fn build_document_updated_event(
    resolved: &ResolvedDocument<'_>,
    sync_run_id: String,
    source_id: String,
    content_id: String,
    doc_url: String,
    user_email: Option<&str>,
) -> ConnectorEvent {
    // Reuse the created event builder and convert the type.
    match build_document_created_event(resolved, sync_run_id, source_id, content_id, doc_url, user_email) {
        ConnectorEvent::DocumentCreated {
            sync_run_id,
            source_id,
            document_id,
            content_id,
            metadata,
            permissions,
            attributes,
        } => ConnectorEvent::DocumentUpdated {
            sync_run_id,
            source_id,
            document_id,
            content_id,
            metadata,
            permissions: Some(permissions),
            attributes,
        },
        other => other,
    }
}

/// Build a `ConnectorEvent::DocumentDeleted` for a removed document.
pub fn build_document_deleted_event(
    sync_run_id: String,
    source_id: String,
    doc_id: i64,
) -> ConnectorEvent {
    ConnectorEvent::DocumentDeleted {
        sync_run_id,
        source_id: source_id.clone(),
        document_id: make_document_id(&source_id, doc_id),
    }
}

/// Lookup tables for resolving Paperless-ngx integer IDs to display names.
pub struct Lookups {
    pub correspondents: HashMap<i64, String>,
    pub document_types: HashMap<i64, String>,
    pub tags: HashMap<i64, String>,
    pub storage_paths: HashMap<i64, String>,
}

impl Lookups {
    pub fn resolve<'a>(&'a self, doc: &'a PaperlessDocument) -> ResolvedDocument<'a> {
        ResolvedDocument {
            doc,
            correspondent_name: doc
                .correspondent
                .and_then(|id| self.correspondents.get(&id))
                .map(String::as_str),
            document_type_name: doc
                .document_type
                .and_then(|id| self.document_types.get(&id))
                .map(String::as_str),
            tag_names: doc
                .tags
                .iter()
                .filter_map(|id| self.tags.get(id))
                .map(String::as_str)
                .collect(),
            storage_path_name: doc
                .storage_path
                .and_then(|id| self.storage_paths.get(&id))
                .map(String::as_str),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{DocumentNote, PaperlessDocument};

    fn make_doc(id: i64, title: &str, content: &str) -> PaperlessDocument {
        PaperlessDocument {
            id,
            title: title.to_string(),
            content: content.to_string(),
            correspondent: None,
            document_type: None,
            storage_path: None,
            tags: vec![],
            created: Some("2024-01-15T10:00:00Z".to_string()),
            modified: Some("2024-01-16T10:00:00Z".to_string()),
            added: Some("2024-01-16T10:00:00Z".to_string()),
            archive_serial_number: None,
            original_file_name: Some("document.pdf".to_string()),
            archived_file_name: None,
            notes: vec![],
        }
    }

    #[test]
    fn test_make_document_id() {
        assert_eq!(make_document_id("src-123", 42), "paperless:src-123:42");
    }

    #[test]
    fn test_generate_document_content_full() {
        let doc = PaperlessDocument {
            id: 1,
            title: "Invoice 2024".to_string(),
            content: "Total: $500".to_string(),
            correspondent: Some(1),
            document_type: Some(2),
            storage_path: None,
            tags: vec![1, 2],
            created: Some("2024-01-15T10:00:00Z".to_string()),
            modified: Some("2024-01-16T10:00:00Z".to_string()),
            added: Some("2024-01-16T10:00:00Z".to_string()),
            archive_serial_number: Some("ASN-001".to_string()),
            original_file_name: Some("invoice.pdf".to_string()),
            archived_file_name: None,
            notes: vec![DocumentNote { note: "Paid".to_string() }],
        };

        let lookups = Lookups {
            correspondents: HashMap::from([(1, "Alice Bank".to_string())]),
            document_types: HashMap::from([(2, "Invoice".to_string())]),
            tags: HashMap::from([(1, "bills".to_string()), (2, "important".to_string())]),
            storage_paths: HashMap::new(),
        };

        let resolved = lookups.resolve(&doc);
        let content = generate_document_content(&resolved);

        assert!(content.contains("# Invoice 2024"));
        assert!(content.contains("**Correspondent:** Alice Bank"));
        assert!(content.contains("**Document Type:** Invoice"));
        assert!(content.contains("**Tags:**"));
        assert!(content.contains("bills") || content.contains("important"));
        assert!(content.contains("**Archive Serial Number:** ASN-001"));
        assert!(content.contains("**File:** invoice.pdf"));
        assert!(content.contains("Total: $500"));
        assert!(content.contains("**Notes:**"));
        assert!(content.contains("Paid"));
    }

    #[test]
    fn test_generate_document_content_minimal() {
        let doc = make_doc(1, "Simple Doc", "Some content here");
        let lookups = Lookups {
            correspondents: HashMap::new(),
            document_types: HashMap::new(),
            tags: HashMap::new(),
            storage_paths: HashMap::new(),
        };
        let resolved = lookups.resolve(&doc);
        let content = generate_document_content(&resolved);

        assert!(content.contains("# Simple Doc"));
        assert!(content.contains("Some content here"));
        // No correspondent or document type should be in output
        assert!(!content.contains("Correspondent:"));
        assert!(!content.contains("Document Type:"));
    }

    #[test]
    fn test_generate_document_content_empty_content() {
        let doc = make_doc(1, "Empty Doc", "");
        let lookups = Lookups {
            correspondents: HashMap::new(),
            document_types: HashMap::new(),
            tags: HashMap::new(),
            storage_paths: HashMap::new(),
        };
        let resolved = lookups.resolve(&doc);
        let content = generate_document_content(&resolved);
        assert!(content.contains("# Empty Doc"));
        // No separator should appear when content is empty
        assert!(!content.contains("---\n\n\n"));
    }

    #[test]
    fn test_connector_state_round_trip() {
        let mut state = PaperlessConnectorState::default();
        state.indexed_ids.insert(1);
        state.indexed_ids.insert(42);
        state.modified_at.insert(1, "2024-01-15T10:00:00Z".to_string());
        state.modified_at.insert(42, "2024-02-01T12:00:00Z".to_string());

        let json = state.to_json();
        let restored = PaperlessConnectorState::from_connector_state(&Some(json));
        assert!(restored.indexed_ids.contains(&1));
        assert!(restored.indexed_ids.contains(&42));
        assert_eq!(restored.modified_at[&1], "2024-01-15T10:00:00Z");
        assert_eq!(restored.modified_at[&42], "2024-02-01T12:00:00Z");
    }

    #[test]
    fn test_connector_state_from_none() {
        let state = PaperlessConnectorState::from_connector_state(&None);
        assert!(state.indexed_ids.is_empty());
        assert!(state.modified_at.is_empty());
    }

    #[test]
    fn test_build_document_created_event() {
        let doc = PaperlessDocument {
            id: 5,
            title: "Test Doc".to_string(),
            content: "Content here".to_string(),
            correspondent: Some(1),
            document_type: None,
            storage_path: None,
            tags: vec![],
            created: Some("2024-01-01T00:00:00Z".to_string()),
            modified: Some("2024-01-02T00:00:00Z".to_string()),
            added: Some("2024-01-02T00:00:00Z".to_string()),
            archive_serial_number: None,
            original_file_name: None,
            archived_file_name: None,
            notes: vec![],
        };
        let lookups = Lookups {
            correspondents: HashMap::from([(1, "John Doe".to_string())]),
            document_types: HashMap::new(),
            tags: HashMap::new(),
            storage_paths: HashMap::new(),
        };
        let resolved = lookups.resolve(&doc);

        let event = build_document_created_event(
            &resolved,
            "sync-1".to_string(),
            "source-1".to_string(),
            "content-1".to_string(),
            "http://paperless/documents/5/details".to_string(),
            Some("admin@example.com"),
        );

        match event {
            ConnectorEvent::DocumentCreated {
                document_id,
                metadata,
                permissions,
                attributes,
                ..
            } => {
                assert_eq!(document_id, "paperless:source-1:5");
                assert_eq!(metadata.title, Some("Test Doc".to_string()));
                assert_eq!(metadata.author, Some("John Doe".to_string()));
                assert_eq!(permissions.users, vec!["admin@example.com"]);
                let attrs = attributes.unwrap();
                assert_eq!(attrs.get("correspondent").unwrap(), "John Doe");
            }
            _ => panic!("Expected DocumentCreated"),
        }
    }

    #[test]
    fn test_build_document_deleted_event() {
        let event = build_document_deleted_event("sync-1".to_string(), "source-1".to_string(), 99);
        match event {
            ConnectorEvent::DocumentDeleted { document_id, .. } => {
                assert_eq!(document_id, "paperless:source-1:99");
            }
            _ => panic!("Expected DocumentDeleted"),
        }
    }

    #[test]
    fn test_parse_datetime_valid() {
        let dt = parse_datetime("2024-01-15T10:00:00Z");
        assert!(dt.is_some());
    }

    #[test]
    fn test_parse_datetime_invalid() {
        let dt = parse_datetime("not-a-date");
        assert!(dt.is_none());
    }
}
