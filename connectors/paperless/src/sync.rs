use anyhow::{anyhow, Context, Result};
use dashmap::DashMap;
use shared::models::{ServiceProvider, SourceType, SyncRequest};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::{error, info, warn};

use crate::client::PaperlessClient;
use crate::config::PaperlessConfig;
use crate::models::PaperlessConnectorState;
use shared::SdkClient;

pub struct SyncManager {
    sdk_client: SdkClient,
    client: PaperlessClient,
    active_syncs: DashMap<String, Arc<AtomicBool>>,
}

impl SyncManager {
    pub fn new(sdk_client: SdkClient) -> Self {
        Self {
            sdk_client,
            client: PaperlessClient::new(),
            active_syncs: DashMap::new(),
        }
    }

    pub fn cancel_sync(&self, sync_run_id: &str) -> bool {
        if let Some(cancelled) = self.active_syncs.get(sync_run_id) {
            cancelled.store(true, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    pub async fn sync_source(&self, request: SyncRequest) -> Result<()> {
        let sync_run_id = &request.sync_run_id;

        if let Err(e) = self.do_sync(&request).await {
            let _ = self.sdk_client.fail(sync_run_id, &e.to_string()).await;
            return Err(e);
        }
        Ok(())
    }

    async fn do_sync(&self, request: &SyncRequest) -> Result<()> {
        let sync_run_id = &request.sync_run_id;
        let source_id = &request.source_id;

        info!(
            "Starting paperless-ngx sync for source {} (run: {})",
            source_id, sync_run_id
        );

        let source = self.sdk_client.get_source(source_id).await?;
        if !source.is_active {
            return Err(anyhow!("Source is not active: {}", source_id));
        }
        if source.source_type != SourceType::PaperlessNgx {
            return Err(anyhow!(
                "Invalid source type for paperless connector: {:?}",
                source.source_type
            ));
        }

        let creds = self.sdk_client.get_credentials(source_id).await?;
        if creds.provider != ServiceProvider::PaperlessNgx {
            return Err(anyhow!(
                "Expected paperless-ngx credentials, found {:?}",
                creds.provider
            ));
        }

        let api_key = creds
            .credentials
            .get("api_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'api_key' in credentials"))?
            .to_string();

        let config = PaperlessConfig::from_source_config(&source.config)?;

        if !config.sync_enabled {
            info!("Sync disabled for source {}, skipping", source_id);
            let _ = self.sdk_client.complete(sync_run_id, 0, 0, None).await;
            return Ok(());
        }

        let is_full = request.sync_mode == "full";
        let mut state = PaperlessConnectorState::from_connector_state(&source.connector_state);
        if is_full {
            info!("Full sync: resetting connector state for source {}", source_id);
            state = PaperlessConnectorState::default();
        }

        // Clone so the borrow of `state.last_sync_at` (immutable) doesn't
        // conflict with the mutable borrow of `state` passed to `execute_sync`.
        let modified_after: Option<String> = if is_full {
            None
        } else {
            state.last_sync_at.clone()
        };

        let cancelled = Arc::new(AtomicBool::new(false));
        self.active_syncs
            .insert(sync_run_id.to_string(), cancelled.clone());

        let result = self
            .execute_sync(
                &config.base_url,
                &api_key,
                source_id,
                sync_run_id,
                modified_after.as_deref(),
                is_full,
                &mut state,
                &cancelled,
            )
            .await;

        if cancelled.load(Ordering::SeqCst) {
            info!("Paperless sync {} was cancelled", sync_run_id);
            let _ = self
                .sdk_client
                .save_connector_state(source_id, state.to_json())
                .await;
            let _ = self.sdk_client.cancel(sync_run_id).await;
            self.active_syncs.remove(sync_run_id);
            return Ok(());
        }

        self.active_syncs.remove(sync_run_id);

        match result {
            Ok(processed) => {
                info!(
                    "Paperless sync completed for source {}: {} documents processed",
                    source_id, processed
                );
                // Record completion time for next incremental sync
                state.last_sync_at = Some(
                    OffsetDateTime::now_utc()
                        .format(&time::format_description::well_known::Rfc3339)
                        .unwrap_or_default(),
                );
                let _ = self
                    .sdk_client
                    .complete(sync_run_id, processed as i32, processed as i32, Some(state.to_json()))
                    .await;
                Ok(())
            }
            Err(e) => {
                error!("Paperless sync failed for source {}: {}", source_id, e);
                Err(e)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_sync(
        &self,
        base_url: &str,
        api_key: &str,
        source_id: &str,
        sync_run_id: &str,
        modified_after: Option<&str>,
        is_full: bool,
        state: &mut PaperlessConnectorState,
        cancelled: &AtomicBool,
    ) -> Result<u32> {
        // Fetch metadata lookups in parallel (tags, correspondents, doc types)
        let (tags_res, correspondents_res, doc_types_res) = tokio::join!(
            self.client.fetch_tags(base_url, api_key),
            self.client.fetch_correspondents(base_url, api_key),
            self.client.fetch_document_types(base_url, api_key),
        );

        let tag_names = tags_res.unwrap_or_else(|e| {
            warn!("Failed to fetch tags: {}", e);
            Default::default()
        });
        let correspondent_names = correspondents_res.unwrap_or_else(|e| {
            warn!("Failed to fetch correspondents: {}", e);
            Default::default()
        });
        let document_type_names = doc_types_res.unwrap_or_else(|e| {
            warn!("Failed to fetch document types: {}", e);
            Default::default()
        });

        // Fetch documents (all or incremental)
        let documents = self
            .client
            .fetch_all_documents(base_url, api_key, modified_after)
            .await
            .context("Failed to fetch documents from paperless-ngx")?;

        let total = documents.len();
        info!("Fetched {} documents to process", total);

        // On full sync, detect deletions before processing new docs
        if is_full && !state.indexed_ids.is_empty() {
            let current_ids: std::collections::HashSet<i64> =
                documents.iter().map(|d| d.id).collect();
            let deleted: Vec<i64> = state
                .indexed_ids
                .iter()
                .copied()
                .filter(|id| !current_ids.contains(id))
                .collect();

            for doc_id in &deleted {
                if cancelled.load(Ordering::SeqCst) {
                    return Ok(0);
                }
                let document_id = format!("paperless:{}:{}", source_id, doc_id);
                let event = shared::models::ConnectorEvent::DocumentDeleted {
                    sync_run_id: sync_run_id.to_string(),
                    source_id: source_id.to_string(),
                    document_id,
                };
                if let Err(e) = self
                    .sdk_client
                    .emit_event(sync_run_id, source_id, event)
                    .await
                {
                    warn!("Failed to emit delete event for doc {}: {}", doc_id, e);
                }
            }
        }

        let mut processed = 0u32;
        let mut new_ids: Vec<i64> = Vec::with_capacity(documents.len());

        for doc in &documents {
            if cancelled.load(Ordering::SeqCst) {
                info!("Sync cancelled, stopping after {} documents", processed);
                break;
            }

            new_ids.push(doc.id);

            let content =
                doc.generate_content(base_url, &tag_names, &correspondent_names, &document_type_names);

            let content_id = match self
                .sdk_client
                .store_content(sync_run_id, &content)
                .await
                .context("Failed to store document content")
            {
                Ok(id) => id,
                Err(e) => {
                    warn!("Skipping document {}: {}", doc.id, e);
                    continue;
                }
            };

            let event = doc.to_connector_event(
                sync_run_id.to_string(),
                source_id.to_string(),
                content_id,
                base_url,
                &tag_names,
                &correspondent_names,
                &document_type_names,
            );

            if let Err(e) = self
                .sdk_client
                .emit_event(sync_run_id, source_id, event)
                .await
                .context("Failed to emit connector event")
            {
                warn!("Failed to emit event for doc {}: {}", doc.id, e);
                continue;
            }

            processed += 1;

            if processed % 10 == 0 {
                let _ = self.sdk_client.increment_scanned(sync_run_id, 10).await;
            }
        }

        // Flush remaining scanned count
        let remainder = processed % 10;
        if remainder != 0 {
            let _ = self
                .sdk_client
                .increment_scanned(sync_run_id, remainder as i32)
                .await;
        }

        // Update state with current document IDs (full sync only; incremental merges)
        if is_full {
            state.indexed_ids = new_ids;
        } else {
            // Merge newly seen IDs into existing indexed_ids
            let existing: std::collections::HashSet<i64> =
                state.indexed_ids.iter().copied().collect();
            for id in new_ids {
                if !existing.contains(&id) {
                    state.indexed_ids.push(id);
                }
            }
        }

        Ok(processed)
    }
}
