mod common;

use anyhow::Result;
use async_trait::async_trait;
use omni_web_connector::config::WebSourceConfig;
use omni_web_connector::models::WebPage;
use omni_web_connector::sync::{CrawlResult, PageSource};
use shared::models::SyncStatus;
use std::sync::Arc;
use tokio::sync::mpsc;

use common::WebConnectorTestFixture;

/// Mock page source that returns predefined pages
struct MockPageSource {
    pages: Vec<WebPage>,
}

impl MockPageSource {
    fn new(pages: Vec<WebPage>) -> Self {
        Self { pages }
    }

    fn from_html_pages(pages: Vec<(&str, &str)>) -> Self {
        let web_pages: Vec<WebPage> = pages
            .into_iter()
            .filter_map(|(url, html)| WebPage::from_html(url.to_string(), html).ok())
            .collect();
        Self::new(web_pages)
    }
}

#[async_trait]
impl PageSource for MockPageSource {
    async fn crawl(
        &self,
        _config: &WebSourceConfig,
        tx: mpsc::Sender<WebPage>,
    ) -> Result<CrawlResult> {
        for page in &self.pages {
            if tx.send(page.clone()).await.is_err() {
                break;
            }
        }
        Ok(CrawlResult {
            pages_crawled: self.pages.len(),
        })
    }
}

fn create_test_html(title: &str, content: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head><title>{}</title></head>
<body>
    <h1>{}</h1>
    <p>{}</p>
</body>
</html>"#,
        title, title, content
    )
}

#[tokio::test]
async fn test_sync_creates_events_for_crawled_pages() -> Result<()> {
    let fixture = WebConnectorTestFixture::new().await?;

    // Create test data
    let user_id = fixture.create_test_user("crawl_test@example.com").await?;
    let source_id = fixture
        .create_test_source("Test Website", &user_id, "https://example.com")
        .await?;

    // Create sync run (simulates connector-manager creating it)
    let sync_run_id = fixture.create_sync_run(&source_id).await?;

    // Create mock pages
    let mock_pages = MockPageSource::from_html_pages(vec![
        (
            "https://example.com/",
            &create_test_html("Home", "Welcome to our website"),
        ),
        (
            "https://example.com/about",
            &create_test_html("About", "About our company"),
        ),
        (
            "https://example.com/contact",
            &create_test_html("Contact", "Get in touch with us"),
        ),
    ]);

    // Create sync manager with SDK client (tests the full SDK flow)
    let sync_manager = fixture.create_sync_manager(Arc::new(mock_pages));

    // Create sync request
    let sync_request = fixture.create_sync_request(&sync_run_id, &source_id);

    // Trigger sync - this will:
    // 1. Call sdk_client.get_source() -> connector-manager -> database
    // 2. Process pages and emit events via SDK
    sync_manager.sync_source(sync_request).await?;

    // Verify events were created via SDK -> connector-manager -> database
    let events = fixture.get_queued_events(&source_id).await?;
    assert_eq!(events.len(), 3, "Expected 3 events for 3 pages");

    // Verify event structure (flat structure with "type" field)
    for event in &events {
        assert_eq!(
            event["type"].as_str(),
            Some("document_created"),
            "Event should be document_created"
        );
        assert_eq!(event["source_id"], source_id);
        assert!(event["document_id"].as_str().is_some());
        assert!(event["content_id"].as_str().is_some());
        assert!(event["metadata"]["title"].as_str().is_some());
        assert_eq!(event["metadata"]["mime_type"], "text/html");
    }

    // Verify sync run completed
    let sync_run = fixture.get_sync_run(&sync_run_id).await?;
    assert!(sync_run.is_some());
    let sync_run = sync_run.unwrap();
    assert_eq!(sync_run.status, SyncStatus::Completed);
    assert_eq!(sync_run.documents_scanned, 3);
    assert_eq!(sync_run.documents_updated, 3);

    Ok(())
}

#[tokio::test]
async fn test_sync_skips_unchanged_pages_on_resync() -> Result<()> {
    let fixture = WebConnectorTestFixture::new().await?;

    let user_id = fixture.create_test_user("test2@example.com").await?;
    let source_id = fixture
        .create_test_source("Test Website 2", &user_id, "https://example2.com")
        .await?;

    // First sync with initial pages
    let initial_pages = MockPageSource::from_html_pages(vec![
        (
            "https://example2.com/page1",
            &create_test_html("Page 1", "Initial content"),
        ),
        (
            "https://example2.com/page2",
            &create_test_html("Page 2", "More content"),
        ),
    ]);

    let sync_run_id_1 = fixture.create_sync_run(&source_id).await?;
    let sync_manager = fixture.create_sync_manager(Arc::new(initial_pages));
    let sync_request = fixture.create_sync_request(&sync_run_id_1, &source_id);
    sync_manager.sync_source(sync_request).await?;

    let events_after_first_sync = fixture.get_queued_events(&source_id).await?;
    assert_eq!(
        events_after_first_sync.len(),
        2,
        "First sync should create 2 events"
    );

    // Second sync with same pages (unchanged)
    let same_pages = MockPageSource::from_html_pages(vec![
        (
            "https://example2.com/page1",
            &create_test_html("Page 1", "Initial content"),
        ),
        (
            "https://example2.com/page2",
            &create_test_html("Page 2", "More content"),
        ),
    ]);

    let sync_run_id_2 = fixture.create_sync_run(&source_id).await?;
    let sync_manager = fixture.create_sync_manager(Arc::new(same_pages));
    let sync_request = fixture.create_sync_request(&sync_run_id_2, &source_id);
    sync_manager.sync_source(sync_request).await?;

    // Should still have only 2 events (no new events for unchanged pages)
    let events_after_second_sync = fixture.get_queued_events(&source_id).await?;
    assert_eq!(
        events_after_second_sync.len(),
        2,
        "Second sync should not create new events for unchanged pages"
    );

    Ok(())
}

#[tokio::test]
async fn test_sync_creates_events_for_updated_pages() -> Result<()> {
    let fixture = WebConnectorTestFixture::new().await?;

    let user_id = fixture.create_test_user("test3@example.com").await?;
    let source_id = fixture
        .create_test_source("Test Website 3", &user_id, "https://example3.com")
        .await?;

    // First sync
    let initial_pages = MockPageSource::from_html_pages(vec![(
        "https://example3.com/page",
        &create_test_html("Page", "Initial content"),
    )]);

    let sync_run_id_1 = fixture.create_sync_run(&source_id).await?;
    let sync_manager = fixture.create_sync_manager(Arc::new(initial_pages));
    let sync_request = fixture.create_sync_request(&sync_run_id_1, &source_id);
    sync_manager.sync_source(sync_request).await?;

    let events_after_first_sync = fixture.get_queued_events(&source_id).await?;
    assert_eq!(events_after_first_sync.len(), 1);

    // Second sync with updated content
    let updated_pages = MockPageSource::from_html_pages(vec![(
        "https://example3.com/page",
        &create_test_html("Page", "Updated content - this is new!"),
    )]);

    let sync_run_id_2 = fixture.create_sync_run(&source_id).await?;
    let sync_manager = fixture.create_sync_manager(Arc::new(updated_pages));
    let sync_request = fixture.create_sync_request(&sync_run_id_2, &source_id);
    sync_manager.sync_source(sync_request).await?;

    // Should now have 2 events (original + update)
    let events_after_second_sync = fixture.get_queued_events(&source_id).await?;
    assert_eq!(
        events_after_second_sync.len(),
        2,
        "Should create new event for updated page"
    );

    Ok(())
}

#[tokio::test]
async fn test_sync_creates_deletion_events_for_removed_pages() -> Result<()> {
    let fixture = WebConnectorTestFixture::new().await?;

    let user_id = fixture.create_test_user("test4@example.com").await?;
    let source_id = fixture
        .create_test_source("Test Website 4", &user_id, "https://example4.com")
        .await?;

    // First sync with 2 pages
    let initial_pages = MockPageSource::from_html_pages(vec![
        (
            "https://example4.com/keep",
            &create_test_html("Keep", "This stays"),
        ),
        (
            "https://example4.com/remove",
            &create_test_html("Remove", "This will be removed"),
        ),
    ]);

    let sync_run_id_1 = fixture.create_sync_run(&source_id).await?;
    let sync_manager = fixture.create_sync_manager(Arc::new(initial_pages));
    let sync_request = fixture.create_sync_request(&sync_run_id_1, &source_id);
    sync_manager.sync_source(sync_request).await?;

    let events_after_first_sync = fixture.get_queued_events(&source_id).await?;
    assert_eq!(events_after_first_sync.len(), 2);

    // Second sync with only 1 page (one removed)
    let remaining_pages = MockPageSource::from_html_pages(vec![(
        "https://example4.com/keep",
        &create_test_html("Keep", "This stays"),
    )]);

    let sync_run_id_2 = fixture.create_sync_run(&source_id).await?;
    let sync_manager = fixture.create_sync_manager(Arc::new(remaining_pages));
    let sync_request = fixture.create_sync_request(&sync_run_id_2, &source_id);
    sync_manager.sync_source(sync_request).await?;

    // Should have 3 events: 2 creates + 1 delete
    let events_after_second_sync = fixture.get_queued_events(&source_id).await?;
    assert_eq!(
        events_after_second_sync.len(),
        3,
        "Should create deletion event for removed page"
    );

    // Verify the last event is a deletion (flat structure with "type" field)
    let last_event = events_after_second_sync.last().unwrap();
    assert_eq!(
        last_event["type"].as_str(),
        Some("document_deleted"),
        "Last event should be document_deleted"
    );

    Ok(())
}

#[tokio::test]
async fn test_event_contains_correct_metadata() -> Result<()> {
    let fixture = WebConnectorTestFixture::new().await?;

    let user_id = fixture
        .create_test_user("metadata_test@example.com")
        .await?;
    let source_id = fixture
        .create_test_source("Test Website 5", &user_id, "https://example5.com")
        .await?;

    let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Test Page Title</title>
    <meta name="description" content="This is a test description">
</head>
<body>
    <h1>Main Heading</h1>
    <p>Paragraph one with some content.</p>
    <p>Paragraph two with more content.</p>
</body>
</html>"#;

    let mock_pages =
        MockPageSource::from_html_pages(vec![("https://example5.com/test-page", html)]);

    let sync_run_id = fixture.create_sync_run(&source_id).await?;
    let sync_manager = fixture.create_sync_manager(Arc::new(mock_pages));
    let sync_request = fixture.create_sync_request(&sync_run_id, &source_id);
    sync_manager.sync_source(sync_request).await?;

    let events = fixture.get_queued_events(&source_id).await?;
    assert_eq!(events.len(), 1);

    // Events have flat structure with "type" field
    let event = &events[0];
    assert_eq!(event["type"].as_str(), Some("document_created"));
    let metadata = &event["metadata"];

    // Verify metadata fields
    assert_eq!(metadata["title"], "Test Page Title");
    assert_eq!(metadata["mime_type"], "text/html");
    assert_eq!(metadata["url"], "https://example5.com/test-page");
    assert_eq!(metadata["path"], "/test-page");

    // Verify extra metadata
    let extra = &metadata["extra"];
    assert_eq!(extra["domain"], "example5.com");
    assert!(extra["word_count"].as_i64().unwrap() > 0);
    assert!(extra["content_hash"].as_str().is_some());

    Ok(())
}

/// Test that SDK endpoints are being called correctly
#[tokio::test]
async fn test_sdk_integration_source_fetch() -> Result<()> {
    let fixture = WebConnectorTestFixture::new().await?;

    let user_id = fixture.create_test_user("sdk_test@example.com").await?;
    let source_id = fixture
        .create_test_source("SDK Test Website", &user_id, "https://sdk-test.com")
        .await?;

    // Test that SDK client can fetch the source via connector-manager
    let source = fixture.sdk_client.get_source(&source_id).await?;
    assert_eq!(source.id, source_id);
    assert_eq!(source.name, "SDK Test Website");
    assert!(source.is_active);

    Ok(())
}
