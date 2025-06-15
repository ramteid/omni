use crate::AppState;
use anyhow::Result;
use shared::db::repositories::DocumentRepository;
use shared::models::{ConnectorEvent, Document, DocumentMetadata, DocumentPermissions};
use shared::queue::EventQueue;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

pub struct QueueProcessor {
    pub state: AppState,
    pub event_queue: EventQueue,
    pub batch_size: i32,
}

impl QueueProcessor {
    pub fn new(state: AppState) -> Self {
        let event_queue = EventQueue::new(state.db_pool.pool().clone());
        Self {
            state,
            event_queue,
            batch_size: 10,
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting queue processor with batch size: {}", self.batch_size);

        let mut poll_interval = interval(Duration::from_secs(1));
        let mut heartbeat_interval = interval(Duration::from_secs(30));
        let mut retry_interval = interval(Duration::from_secs(300)); // 5 minutes

        loop {
            tokio::select! {
                _ = poll_interval.tick() => {
                    if let Err(e) = self.process_batch().await {
                        error!("Failed to process batch: {}", e);
                    }
                }
                _ = heartbeat_interval.tick() => {
                    if let Ok(stats) = self.event_queue.get_queue_stats().await {
                        info!(
                            "Queue stats - Pending: {}, Processing: {}, Completed: {}, Failed: {}, Dead Letter: {}",
                            stats.pending, stats.processing, stats.completed, stats.failed, stats.dead_letter
                        );
                    }
                }
                _ = retry_interval.tick() => {
                    if let Ok(retried) = self.event_queue.retry_failed_events().await {
                        if retried > 0 {
                            info!("Retried {} failed events", retried);
                        }
                    }
                }
            }
        }
    }

    async fn process_batch(&self) -> Result<()> {
        let events = self.event_queue.dequeue_batch(self.batch_size).await?;
        
        for event_item in events {
            let event_id = event_item.id.clone();
            
            match self.process_event(&event_item.payload).await {
                Ok(_) => {
                    self.event_queue.mark_completed(&event_id).await?;
                }
                Err(e) => {
                    error!("Failed to process event {}: {}", event_id, e);
                    self.event_queue.mark_failed(&event_id, &e.to_string()).await?;
                }
            }
        }
        
        Ok(())
    }

    async fn process_event(&self, payload: &serde_json::Value) -> Result<()> {
        let event: ConnectorEvent = serde_json::from_value(payload.clone())?;

        match event {
            ConnectorEvent::DocumentCreated {
                source_id,
                document_id,
                content,
                metadata,
                permissions,
            } => {
                self.handle_document_created(
                    source_id,
                    document_id,
                    content,
                    metadata,
                    permissions,
                )
                .await?;
            }
            ConnectorEvent::DocumentUpdated {
                source_id,
                document_id,
                content,
                metadata,
                permissions,
            } => {
                self.handle_document_updated(
                    source_id,
                    document_id,
                    content,
                    metadata,
                    permissions,
                )
                .await?;
            }
            ConnectorEvent::DocumentDeleted {
                source_id,
                document_id,
            } => {
                self.handle_document_deleted(source_id, document_id).await?;
            }
        }

        Ok(())
    }

    async fn handle_document_created(
        &self,
        source_id: String,
        document_id: String,
        content: String,
        metadata: DocumentMetadata,
        permissions: DocumentPermissions,
    ) -> Result<()> {
        info!(
            "Processing document created: {} from source {}",
            document_id, source_id
        );

        let now = sqlx::types::time::OffsetDateTime::now_utc();
        let metadata_json = serde_json::to_value(&metadata)?;
        let permissions_json = serde_json::to_value(&permissions)?;

        let document = Document {
            id: ulid::Ulid::new().to_string(),
            source_id: source_id.clone(),
            external_id: document_id.clone(),
            title: metadata.title.unwrap_or_else(|| "Untitled".to_string()),
            content: Some(content.clone()),
            content_type: None,
            file_size: None,
            file_extension: None,
            url: None,
            parent_id: None,
            metadata: metadata_json,
            permissions: permissions_json,
            created_at: now,
            updated_at: now,
            last_indexed_at: now,
        };

        let repo = DocumentRepository::new(self.state.db_pool.pool());
        let created = repo.create(document).await?;

        repo.update_search_vector(&created.id).await?;
        repo.mark_as_indexed(&created.id).await?;

        info!("Document created successfully: {}", document_id);
        Ok(())
    }

    async fn handle_document_updated(
        &self,
        source_id: String,
        document_id: String,
        content: String,
        metadata: DocumentMetadata,
        permissions: Option<DocumentPermissions>,
    ) -> Result<()> {
        info!(
            "Processing document updated: {} from source {}",
            document_id, source_id
        );

        let repo = DocumentRepository::new(self.state.db_pool.pool());

        if let Some(mut document) = repo.find_by_external_id(&source_id, &document_id).await? {
            let now = sqlx::types::time::OffsetDateTime::now_utc();
            let metadata_json = serde_json::to_value(&metadata)?;
            let doc_id = document.id.clone();

            document.title = metadata.title.unwrap_or(document.title);
            document.content = Some(content);
            document.metadata = metadata_json;
            if let Some(perms) = permissions {
                document.permissions = serde_json::to_value(&perms)?;
            }
            document.updated_at = now;

            repo.update(&doc_id, document).await?;
            repo.update_search_vector(&doc_id).await?;
            repo.mark_as_indexed(&doc_id).await?;

            info!("Document updated successfully: {}", document_id);
        } else {
            warn!(
                "Document not found for update: {} from source {}",
                document_id, source_id
            );
        }

        Ok(())
    }

    async fn handle_document_deleted(&self, source_id: String, document_id: String) -> Result<()> {
        info!(
            "Processing document deleted: {} from source {}",
            document_id, source_id
        );

        let repo = DocumentRepository::new(self.state.db_pool.pool());

        if let Some(document) = repo.find_by_external_id(&source_id, &document_id).await? {
            repo.delete(&document.id).await?;
            info!("Document deleted successfully: {}", document_id);
        } else {
            warn!(
                "Document not found for deletion: {} from source {}",
                document_id, source_id
            );
        }

        Ok(())
    }
}