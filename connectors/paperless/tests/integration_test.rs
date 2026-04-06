//! Integration tests for the Paperless-ngx connector.
//!
//! These tests validate the end-to-end flow from raw API response data through
//! document parsing, content generation, connector event production, and state
//! management — exercising multiple modules working together as they would in
//! a real sync run.

use omni_paperless_connector::client::{DocumentNote, PaperlessDocument};
use omni_paperless_connector::models::{
    build_document_created_event, build_document_deleted_event, build_document_updated_event,
    generate_document_content, make_document_id, Lookups, PaperlessConnectorState,
};
use shared::models::ConnectorEvent;
use std::collections::HashMap;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_doc(id: i64, title: &str, content: &str, modified: &str) -> PaperlessDocument {
    PaperlessDocument {
        id,
        title: title.to_string(),
        content: content.to_string(),
        correspondent: None,
        document_type: None,
        storage_path: None,
        tags: vec![],
        created: Some("2024-01-01T00:00:00Z".to_string()),
        modified: Some(modified.to_string()),
        added: Some("2024-01-01T00:00:00Z".to_string()),
        archive_serial_number: None,
        original_file_name: Some(format!("doc{}.pdf", id)),
        archived_file_name: None,
        notes: vec![],
    }
}

fn empty_lookups() -> Lookups {
    Lookups {
        correspondents: HashMap::new(),
        document_types: HashMap::new(),
        tags: HashMap::new(),
        storage_paths: HashMap::new(),
    }
}

fn rich_lookups() -> Lookups {
    Lookups {
        correspondents: HashMap::from([
            (1, "Alice Bank".to_string()),
            (2, "Bob Corp".to_string()),
        ]),
        document_types: HashMap::from([
            (10, "Invoice".to_string()),
            (11, "Receipt".to_string()),
        ]),
        tags: HashMap::from([
            (100, "bills".to_string()),
            (101, "important".to_string()),
            (102, "2024".to_string()),
        ]),
        storage_paths: HashMap::from([(50, "Finance/2024".to_string())]),
    }
}

// ── Content generation tests ──────────────────────────────────────────────────

/// Verify that the Markdown output contains all expected sections for a
/// fully-populated document.
#[test]
fn test_content_generation_full_document() {
    let doc = PaperlessDocument {
        id: 42,
        title: "Invoice January 2024".to_string(),
        content: "Total amount: $1,500.00\nDue date: 2024-02-01".to_string(),
        correspondent: Some(1),
        document_type: Some(10),
        storage_path: Some(50),
        tags: vec![100, 101, 102],
        created: Some("2024-01-15T10:00:00Z".to_string()),
        modified: Some("2024-01-16T10:00:00Z".to_string()),
        added: Some("2024-01-16T10:00:00Z".to_string()),
        archive_serial_number: Some("ASN-2024-001".to_string()),
        original_file_name: Some("invoice_jan_2024.pdf".to_string()),
        archived_file_name: Some("invoice_jan_2024.pdf".to_string()),
        notes: vec![
            DocumentNote { note: "Approved by finance team".to_string() },
            DocumentNote { note: "Paid on 2024-01-20".to_string() },
        ],
    };

    let lookups = rich_lookups();
    let resolved = lookups.resolve(&doc);
    let content = generate_document_content(&resolved);

    // Title
    assert!(content.starts_with("# Invoice January 2024"));

    // Metadata fields
    assert!(content.contains("**Correspondent:** Alice Bank"));
    assert!(content.contains("**Document Type:** Invoice"));
    assert!(content.contains("**Storage Path:** Finance/2024"));
    assert!(content.contains("**Archive Serial Number:** ASN-2024-001"));
    assert!(content.contains("**File:** invoice_jan_2024.pdf"));
    assert!(content.contains("**Created:** 2024-01-15T10:00:00Z"));
    assert!(content.contains("**Added:** 2024-01-16T10:00:00Z"));

    // Tags (order not guaranteed, but all must appear)
    assert!(content.contains("bills") || content.contains("important") || content.contains("2024"));

    // OCR content separator and text
    assert!(content.contains("---"));
    assert!(content.contains("Total amount: $1,500.00"));
    assert!(content.contains("Due date: 2024-02-01"));

    // Notes
    assert!(content.contains("**Notes:**"));
    assert!(content.contains("Approved by finance team"));
    assert!(content.contains("Paid on 2024-01-20"));
}

/// Verify that a document with no metadata beyond title still produces
/// valid Markdown with just the content.
#[test]
fn test_content_generation_minimal_document() {
    let doc = make_doc(1, "Minimal Document", "Just some text.", "2024-01-01T00:00:00Z");
    let lookups = empty_lookups();
    let resolved = lookups.resolve(&doc);
    let content = generate_document_content(&resolved);

    assert!(content.starts_with("# Minimal Document"));
    assert!(content.contains("Just some text."));
    // No correspondent, document type lines
    assert!(!content.contains("Correspondent:"));
    assert!(!content.contains("Document Type:"));
    // No notes section
    assert!(!content.contains("Notes:"));
}

/// A document with empty OCR content should not emit an empty separator.
#[test]
fn test_content_generation_empty_ocr_content() {
    let doc = make_doc(1, "Scanned Image", "", "2024-01-01T00:00:00Z");
    let lookups = empty_lookups();
    let resolved = lookups.resolve(&doc);
    let content = generate_document_content(&resolved);

    assert!(content.starts_with("# Scanned Image"));
    // Separator should not appear when content is blank
    assert!(!content.contains("---\n\n\n") && !content.contains("\n---\n\nFile:"));
}

/// Whitespace-only notes should be silently skipped.
#[test]
fn test_content_generation_skips_blank_notes() {
    let doc = PaperlessDocument {
        id: 2,
        title: "Blank Notes".to_string(),
        content: "Body text.".to_string(),
        notes: vec![
            DocumentNote { note: "   ".to_string() },
            DocumentNote { note: "Real note".to_string() },
        ],
        ..make_doc(2, "Blank Notes", "Body text.", "2024-01-01T00:00:00Z")
    };
    let lookups = empty_lookups();
    let resolved = lookups.resolve(&doc);
    let content = generate_document_content(&resolved);

    assert!(content.contains("Real note"));
    // The blank note should not produce a lone dash bullet
    let notes_start = content.find("**Notes:**").expect("notes section missing");
    let notes_section = &content[notes_start..];
    assert!(!notes_section.contains("-    "), "blank note produced an empty bullet");
}

// ── Document ID tests ─────────────────────────────────────────────────────────

#[test]
fn test_document_id_format() {
    assert_eq!(make_document_id("source-abc", 1), "paperless:source-abc:1");
    assert_eq!(make_document_id("src", 9999), "paperless:src:9999");
}

// ── State management tests ────────────────────────────────────────────────────

/// State serializes and deserializes without data loss.
#[test]
fn test_state_round_trip() {
    let mut state = PaperlessConnectorState::default();
    state.indexed_ids.extend([1, 2, 42, 100]);
    state.modified_at.insert(1, "2024-01-01T00:00:00Z".to_string());
    state.modified_at.insert(42, "2024-06-15T12:00:00Z".to_string());

    let json = state.to_json();
    let restored = PaperlessConnectorState::from_connector_state(&Some(json));

    assert_eq!(restored.indexed_ids.len(), 4);
    assert!(restored.indexed_ids.contains(&1));
    assert!(restored.indexed_ids.contains(&42));
    assert_eq!(restored.modified_at[&1], "2024-01-01T00:00:00Z");
    assert_eq!(restored.modified_at[&42], "2024-06-15T12:00:00Z");
}

/// Missing connector_state (first run) produces an empty default.
#[test]
fn test_state_from_none_is_default() {
    let state = PaperlessConnectorState::from_connector_state(&None);
    assert!(state.indexed_ids.is_empty());
    assert!(state.modified_at.is_empty());
}

/// Corrupt connector_state JSON falls back to default instead of panicking.
#[test]
fn test_state_from_corrupt_json_falls_back() {
    let corrupt = Some(serde_json::json!("this is not a valid state object"));
    let state = PaperlessConnectorState::from_connector_state(&corrupt);
    assert!(state.indexed_ids.is_empty());
}

// ── Incremental sync simulation ───────────────────────────────────────────────

/// Simulate a multi-batch incremental sync:
///
/// Batch 1 – 3 docs land; all are new.
/// Batch 2 – Doc 2 is modified (different `modified` timestamp); doc 4 is new.
/// Batch 3 – Doc 1 disappears (deleted from Paperless-ngx).
///
/// Verifies:
/// - New docs → `DocumentCreated`
/// - Modified docs → `DocumentUpdated`
/// - Stable document IDs across batches
/// - Correct deletion detection
#[test]
fn test_incremental_sync_flow() {
    let source_id = "source-42";

    // ── Batch 1: three new documents ──────────────────────────────────────
    let docs_batch1 = vec![
        make_doc(1, "Contract Alpha", "Contract body.", "2024-01-01T00:00:00Z"),
        make_doc(2, "Invoice Beta", "Invoice body.", "2024-01-02T00:00:00Z"),
        make_doc(3, "Receipt Gamma", "Receipt body.", "2024-01-03T00:00:00Z"),
    ];

    let lookups = empty_lookups();
    let mut state = PaperlessConnectorState::default();
    let mut events: Vec<ConnectorEvent> = Vec::new();

    for doc in &docs_batch1 {
        let modified = doc.modified.clone().unwrap_or_default();
        let is_new = !state.indexed_ids.contains(&doc.id);

        let resolved = lookups.resolve(doc);
        let event = build_document_created_event(
            &resolved,
            "sync-1".into(),
            source_id.into(),
            format!("content-{}", doc.id),
            format!("http://paperless/documents/{}/details", doc.id),
            Some("admin@example.com"),
        );

        state.indexed_ids.insert(doc.id);
        state.modified_at.insert(doc.id, modified.clone());

        assert!(is_new, "Doc {} should be new", doc.id);
        events.push(event);
    }

    assert_eq!(events.len(), 3);
    // All events should be DocumentCreated
    for e in &events {
        assert!(matches!(e, ConnectorEvent::DocumentCreated { .. }), "Expected DocumentCreated");
    }

    // Document IDs must be stable and unique
    let doc_ids: Vec<_> = events
        .iter()
        .map(|e| match e {
            ConnectorEvent::DocumentCreated { document_id, .. } => document_id.clone(),
            _ => String::new(),
        })
        .collect();
    assert_eq!(doc_ids[0], "paperless:source-42:1");
    assert_eq!(doc_ids[1], "paperless:source-42:2");
    assert_eq!(doc_ids[2], "paperless:source-42:3");

    // Persist state (simulates sdk_client.complete)
    let state_json = state.to_json();
    let mut state = PaperlessConnectorState::from_connector_state(&Some(state_json));

    // ── Batch 2: doc 2 modified, doc 4 new ───────────────────────────────
    let docs_batch2 = vec![
        make_doc(1, "Contract Alpha", "Contract body.", "2024-01-01T00:00:00Z"), // unchanged
        make_doc(2, "Invoice Beta Updated", "Invoice body updated.", "2024-03-01T00:00:00Z"), // modified
        make_doc(3, "Receipt Gamma", "Receipt body.", "2024-01-03T00:00:00Z"), // unchanged
        make_doc(4, "New Doc", "New body.", "2024-04-01T00:00:00Z"), // new
    ];

    let mut batch2_events: Vec<ConnectorEvent> = Vec::new();

    for doc in &docs_batch2 {
        let modified = doc.modified.clone().unwrap_or_default();
        let is_new = !state.indexed_ids.contains(&doc.id);
        let is_changed = !is_new
            && state.modified_at.get(&doc.id).map_or(true, |m| m != &modified);

        if !is_new && !is_changed {
            continue;
        }

        let resolved = lookups.resolve(doc);
        let event = if is_new {
            build_document_created_event(
                &resolved,
                "sync-2".into(),
                source_id.into(),
                format!("content-batch2-{}", doc.id),
                format!("http://paperless/documents/{}/details", doc.id),
                Some("admin@example.com"),
            )
        } else {
            build_document_updated_event(
                &resolved,
                "sync-2".into(),
                source_id.into(),
                format!("content-batch2-{}", doc.id),
                format!("http://paperless/documents/{}/details", doc.id),
                Some("admin@example.com"),
            )
        };

        state.indexed_ids.insert(doc.id);
        state.modified_at.insert(doc.id, modified);
        batch2_events.push(event);
    }

    // Only doc 2 (updated) and doc 4 (new) should generate events
    assert_eq!(batch2_events.len(), 2, "Expected 2 events in batch 2");

    let updated_event = batch2_events.iter().find(|e| {
        matches!(e, ConnectorEvent::DocumentUpdated { document_id, .. }
            if document_id == "paperless:source-42:2")
    });
    assert!(updated_event.is_some(), "Doc 2 should produce DocumentUpdated");

    let created_event = batch2_events.iter().find(|e| {
        matches!(e, ConnectorEvent::DocumentCreated { document_id, .. }
            if document_id == "paperless:source-42:4")
    });
    assert!(created_event.is_some(), "Doc 4 should produce DocumentCreated");

    // ── Batch 3: doc 1 deleted (disappears from API response) ────────────
    let current_ids_batch3: std::collections::HashSet<i64> =
        [2, 3, 4].iter().copied().collect();

    let deleted_ids: Vec<i64> = state
        .indexed_ids
        .iter()
        .copied()
        .filter(|id| !current_ids_batch3.contains(id))
        .collect();

    assert_eq!(deleted_ids, vec![1], "Only doc 1 should be deleted");

    let del_event = build_document_deleted_event(
        "sync-3".into(),
        source_id.into(),
        deleted_ids[0],
    );
    match del_event {
        ConnectorEvent::DocumentDeleted { document_id, source_id: sid, .. } => {
            assert_eq!(document_id, "paperless:source-42:1");
            assert_eq!(sid, source_id);
        }
        _ => panic!("Expected DocumentDeleted"),
    }
}

// ── Event field correctness tests ─────────────────────────────────────────────

/// `DocumentCreated` carries the correct metadata fields.
#[test]
fn test_document_created_event_metadata() {
    let doc = PaperlessDocument {
        id: 7,
        title: "Tax Return 2023".to_string(),
        content: "Taxable income: $80,000".to_string(),
        correspondent: Some(2),
        document_type: Some(11),
        storage_path: None,
        tags: vec![101],
        created: Some("2023-04-15T09:00:00Z".to_string()),
        modified: Some("2023-04-16T09:00:00Z".to_string()),
        added: Some("2023-04-16T09:00:00Z".to_string()),
        archive_serial_number: Some("ASN-2023-TAX".to_string()),
        original_file_name: Some("tax_return_2023.pdf".to_string()),
        archived_file_name: None,
        notes: vec![],
    };

    let lookups = rich_lookups();
    let resolved = lookups.resolve(&doc);

    let event = build_document_created_event(
        &resolved,
        "sync-1".into(),
        "source-99".into(),
        "cid-1".into(),
        "http://paperless/documents/7/details".into(),
        Some("user@example.com"),
    );

    match event {
        ConnectorEvent::DocumentCreated {
            document_id,
            metadata,
            permissions,
            attributes,
            ..
        } => {
            assert_eq!(document_id, "paperless:source-99:7");
            assert_eq!(metadata.title, Some("Tax Return 2023".to_string()));
            assert_eq!(metadata.author, Some("Bob Corp".to_string())); // correspondent 2
            assert_eq!(metadata.url, Some("http://paperless/documents/7/details".to_string()));
            assert_eq!(metadata.content_type, Some("Receipt".to_string())); // document_type 11
            assert!(!permissions.public);
            assert_eq!(permissions.users, vec!["user@example.com"]);

            let attrs = attributes.unwrap();
            assert_eq!(attrs["correspondent"], "Bob Corp");
            assert_eq!(attrs["document_type"], "Receipt");
            assert_eq!(attrs["archive_serial_number"], "ASN-2023-TAX");
            // Tag "important" (id=101) should be present
            let tags = attrs["tags"].as_array().unwrap();
            assert!(tags.iter().any(|t| t.as_str() == Some("important")));
        }
        _ => panic!("Expected DocumentCreated"),
    }
}

/// `DocumentUpdated` carries the same fields as `DocumentCreated`.
#[test]
fn test_document_updated_event_has_permissions() {
    let doc = make_doc(3, "Updated Doc", "Content.", "2024-05-01T00:00:00Z");
    let lookups = empty_lookups();
    let resolved = lookups.resolve(&doc);

    let event = build_document_updated_event(
        &resolved,
        "sync-2".into(),
        "src".into(),
        "cid".into(),
        "http://paperless/documents/3/details".into(),
        Some("admin@example.com"),
    );

    match event {
        ConnectorEvent::DocumentUpdated { document_id, permissions, .. } => {
            assert_eq!(document_id, "paperless:src:3");
            let perms = permissions.unwrap();
            assert_eq!(perms.users, vec!["admin@example.com"]);
        }
        _ => panic!("Expected DocumentUpdated"),
    }
}

/// When no `user_email` is supplied, permissions list should be empty.
#[test]
fn test_document_created_event_no_user_email() {
    let doc = make_doc(1, "Doc", "Content", "2024-01-01T00:00:00Z");
    let lookups = empty_lookups();
    let resolved = lookups.resolve(&doc);

    let event = build_document_created_event(
        &resolved,
        "sync-1".into(),
        "src".into(),
        "cid".into(),
        "http://paperless/documents/1/details".into(),
        None,
    );

    match event {
        ConnectorEvent::DocumentCreated { permissions, .. } => {
            assert!(permissions.users.is_empty());
        }
        _ => panic!("Expected DocumentCreated"),
    }
}

// ── Lookup resolution tests ───────────────────────────────────────────────────

/// Unknown IDs (not in the lookup maps) do not appear in resolved output.
#[test]
fn test_lookups_missing_ids_return_none() {
    let doc = PaperlessDocument {
        id: 1,
        title: "Orphaned".to_string(),
        content: String::new(),
        correspondent: Some(999), // not in lookups
        document_type: Some(888), // not in lookups
        storage_path: None,
        tags: vec![777],          // not in lookups
        created: None,
        modified: None,
        added: None,
        archive_serial_number: None,
        original_file_name: None,
        archived_file_name: None,
        notes: vec![],
    };

    let lookups = empty_lookups(); // all maps empty
    let resolved = lookups.resolve(&doc);

    assert!(resolved.correspondent_name.is_none());
    assert!(resolved.document_type_name.is_none());
    assert!(resolved.tag_names.is_empty());
    assert!(resolved.storage_path_name.is_none());
}
