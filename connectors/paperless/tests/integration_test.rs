//! Integration tests for the paperless-ngx connector.
//!
//! These tests validate content generation, state management, and event
//! construction without requiring a live paperless-ngx instance.

use omni_paperless_connector::models::{
    PaperlessConnectorState, PaperlessDocument, PaperlessLabel, PaginatedResponse,
};
use shared::models::ConnectorEvent;
use std::collections::HashMap;

// ── Helpers ─────────────────────────────────────────────────────────────────

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
        original_file_name: Some(format!("doc_{}.pdf", id)),
        archived_file_name: None,
        owner: Some("admin".to_string()),
    }
}

fn make_doc_with_metadata(
    id: i64,
    title: &str,
    tags: Vec<i64>,
    correspondent: Option<i64>,
    document_type: Option<i64>,
) -> PaperlessDocument {
    PaperlessDocument {
        id,
        title: title.to_string(),
        content: format!("Content of document {}", id),
        tags,
        correspondent,
        document_type,
        storage_path: None,
        created: Some("2024-03-01T08:00:00Z".to_string()),
        modified: Some("2024-03-10T15:00:00Z".to_string()),
        added: Some("2024-03-01T08:30:00Z".to_string()),
        original_file_name: Some(format!("{}.pdf", title.to_lowercase().replace(' ', "_"))),
        archived_file_name: None,
        owner: Some("user@example.com".to_string()),
    }
}

// ── Content generation ───────────────────────────────────────────────────────

#[test]
fn test_content_generation_basic() {
    let doc = make_doc(1, "My Invoice", "This is the invoice text.");
    let content = doc.generate_content(
        "http://localhost:8000",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    );

    assert!(content.starts_with("# My Invoice\n"), "Missing title header");
    assert!(content.contains("**Date Created:** 2024-01-15T10:00:00Z"));
    assert!(content.contains("**Modified:** 2024-01-16T12:00:00Z"));
    assert!(content.contains("**Original File:** doc_1.pdf"));
    assert!(content.contains("**Owner:** admin"));
    assert!(content.contains("http://localhost:8000/documents/1/details"));
    assert!(content.contains("---"), "Missing separator before OCR content");
    assert!(content.contains("This is the invoice text."));
}

#[test]
fn test_content_generation_with_tags_correspondent_doctype() {
    let doc = make_doc_with_metadata(2, "Annual Report", vec![10, 20], Some(5), Some(3));

    let mut tags = HashMap::new();
    tags.insert(10i64, "finance".to_string());
    tags.insert(20i64, "annual".to_string());

    let mut correspondents = HashMap::new();
    correspondents.insert(5i64, "Big Corp".to_string());

    let mut doc_types = HashMap::new();
    doc_types.insert(3i64, "Report".to_string());

    let content = doc.generate_content("http://paperless.local", &tags, &correspondents, &doc_types);

    assert!(content.contains("**Correspondent:** Big Corp"));
    assert!(content.contains("**Document Type:** Report"));
    let tag_line = content
        .lines()
        .find(|l| l.starts_with("**Tags:**"))
        .expect("Tags line not found");
    assert!(tag_line.contains("finance"));
    assert!(tag_line.contains("annual"));
}

#[test]
fn test_content_generation_empty_content_no_separator() {
    let doc = make_doc(3, "Empty", "");
    let content = doc.generate_content(
        "http://localhost:8000",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    );
    // No separator should be added when the OCR content is empty
    assert!(!content.contains("---"), "Separator should be absent for empty content");
}

#[test]
fn test_content_generation_only_whitespace_content() {
    let doc = make_doc(4, "Whitespace", "   \n\t  \n   ");
    let content = doc.generate_content(
        "http://localhost:8000",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    );
    assert!(!content.contains("---"), "Separator should be absent when content is all whitespace");
}

#[test]
fn test_content_generation_missing_metadata_fields() {
    let doc = PaperlessDocument {
        id: 99,
        title: "Sparse Document".to_string(),
        content: "Some text".to_string(),
        tags: vec![],
        correspondent: None,
        document_type: None,
        storage_path: None,
        created: None,
        modified: None,
        added: None,
        original_file_name: None,
        archived_file_name: None,
        owner: None,
    };

    let content = doc.generate_content(
        "http://localhost:8000",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    );

    assert!(content.contains("# Sparse Document"));
    // None fields must not appear
    assert!(!content.contains("**Date Created:**"));
    assert!(!content.contains("**Modified:**"));
    assert!(!content.contains("**Owner:**"));
    assert!(!content.contains("**Original File:**"));
    // Content still present
    assert!(content.contains("Some text"));
}

// ── External ID & URL ────────────────────────────────────────────────────────

#[test]
fn test_external_id_is_stable_and_unique() {
    let doc1 = make_doc(1, "Doc A", "");
    let doc2 = make_doc(2, "Doc B", "");

    let id1 = doc1.external_id("source-1");
    let id2 = doc2.external_id("source-1");

    assert_eq!(id1, "paperless:source-1:1");
    assert_eq!(id2, "paperless:source-1:2");
    assert_ne!(id1, id2);
}

#[test]
fn test_external_id_differs_by_source() {
    let doc = make_doc(1, "Doc", "");
    assert_ne!(doc.external_id("source-a"), doc.external_id("source-b"));
}

#[test]
fn test_document_url_includes_id() {
    let doc = make_doc(42, "Doc", "");
    let url = doc.document_url("https://paperless.myco.com");
    assert_eq!(url, "https://paperless.myco.com/documents/42/details");
}

// ── ConnectorEvent construction ──────────────────────────────────────────────

#[test]
fn test_to_connector_event_created() {
    let doc = make_doc(7, "Tax Return 2024", "Taxable income: ...");

    let event = doc.to_connector_event(
        "run-1".to_string(),
        "src-1".to_string(),
        "content-abc".to_string(),
        "http://localhost:8000",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    );

    match event {
        ConnectorEvent::DocumentCreated {
            sync_run_id,
            source_id,
            document_id,
            content_id,
            metadata,
            ..
        } => {
            assert_eq!(sync_run_id, "run-1");
            assert_eq!(source_id, "src-1");
            assert_eq!(document_id, "paperless:src-1:7");
            assert_eq!(content_id, "content-abc");
            assert_eq!(metadata.title, Some("Tax Return 2024".to_string()));
            assert!(metadata.url.as_deref().unwrap().contains("/documents/7/details"));
        }
        other => panic!("Expected DocumentCreated, got {:?}", other),
    }
}

// ── State management ─────────────────────────────────────────────────────────

#[test]
fn test_state_roundtrip() {
    let state = PaperlessConnectorState {
        last_sync_at: Some("2024-06-01T00:00:00Z".to_string()),
        indexed_ids: vec![1, 2, 3, 100],
    };

    let json = state.to_json();
    let restored = PaperlessConnectorState::from_connector_state(&Some(json));

    assert_eq!(restored.last_sync_at, state.last_sync_at);
    assert_eq!(restored.indexed_ids, state.indexed_ids);
}

#[test]
fn test_state_defaults_when_none() {
    let state = PaperlessConnectorState::from_connector_state(&None);
    assert!(state.last_sync_at.is_none());
    assert!(state.indexed_ids.is_empty());
}

#[test]
fn test_state_defaults_on_invalid_json() {
    let state = PaperlessConnectorState::from_connector_state(&Some(serde_json::json!("invalid")));
    assert!(state.last_sync_at.is_none());
    assert!(state.indexed_ids.is_empty());
}

// ── Deletion detection logic ─────────────────────────────────────────────────

#[test]
fn test_deletion_detection_identifies_removed_documents() {
    // Simulate state from a previous full sync: IDs 1-5 were indexed.
    let indexed_ids = vec![1i64, 2, 3, 4, 5];

    // Current API response: IDs 1, 3, 5 (2 and 4 were deleted from paperless-ngx).
    let current_ids: std::collections::HashSet<i64> = [1, 3, 5].iter().copied().collect();

    let deleted: Vec<i64> = indexed_ids
        .iter()
        .copied()
        .filter(|id| !current_ids.contains(id))
        .collect();

    assert!(deleted.contains(&2), "Document 2 should be detected as deleted");
    assert!(deleted.contains(&4), "Document 4 should be detected as deleted");
    assert!(!deleted.contains(&1), "Document 1 should not be deleted");
    assert!(!deleted.contains(&3), "Document 3 should not be deleted");
    assert!(!deleted.contains(&5), "Document 5 should not be deleted");
}

#[test]
fn test_no_deletions_when_all_present() {
    let indexed_ids = vec![1i64, 2, 3];
    let current_ids: std::collections::HashSet<i64> = [1, 2, 3].iter().copied().collect();

    let deleted: Vec<i64> = indexed_ids
        .iter()
        .copied()
        .filter(|id| !current_ids.contains(id))
        .collect();

    assert!(deleted.is_empty());
}

// ── Paginated response deserialization ───────────────────────────────────────

#[test]
fn test_paginated_response_deserialization() {
    let json = serde_json::json!({
        "count": 2,
        "next": null,
        "previous": null,
        "results": [
            {
                "id": 1,
                "title": "Invoice",
                "content": "Invoice text",
                "tags": [1, 2],
                "correspondent": 3,
                "document_type": 4,
                "storage_path": null,
                "created": "2024-01-15T10:00:00Z",
                "modified": "2024-01-16T12:00:00Z",
                "added": "2024-01-16T12:00:00Z",
                "original_file_name": "invoice.pdf",
                "archived_file_name": "0000001.pdf",
                "owner": "admin"
            },
            {
                "id": 2,
                "title": "Contract",
                "content": "",
                "tags": [],
                "correspondent": null,
                "document_type": null,
                "storage_path": null,
                "created": null,
                "modified": "2024-02-01T00:00:00Z",
                "added": "2024-02-01T00:00:00Z",
                "original_file_name": null,
                "archived_file_name": null,
                "owner": null
            }
        ]
    });

    let response: PaginatedResponse<PaperlessDocument> =
        serde_json::from_value(json).expect("Deserialization failed");

    assert_eq!(response.count, 2);
    assert!(response.next.is_none());
    assert_eq!(response.results.len(), 2);

    let doc1 = &response.results[0];
    assert_eq!(doc1.id, 1);
    assert_eq!(doc1.title, "Invoice");
    assert_eq!(doc1.tags, vec![1, 2]);
    assert_eq!(doc1.correspondent, Some(3));
    assert_eq!(doc1.owner.as_deref(), Some("admin"));

    let doc2 = &response.results[1];
    assert_eq!(doc2.id, 2);
    assert!(doc2.content.is_empty());
    assert!(doc2.correspondent.is_none());
}

#[test]
fn test_label_deserialization() {
    let json = serde_json::json!({
        "count": 2,
        "next": null,
        "previous": null,
        "results": [
            { "id": 1, "name": "Finance" },
            { "id": 2, "name": "Legal" }
        ]
    });

    let response: PaginatedResponse<PaperlessLabel> =
        serde_json::from_value(json).expect("Label deserialization failed");

    assert_eq!(response.results.len(), 2);
    assert_eq!(response.results[0].id, 1);
    assert_eq!(response.results[0].name, "Finance");
    assert_eq!(response.results[1].name, "Legal");
}
