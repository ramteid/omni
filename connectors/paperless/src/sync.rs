use anyhow::{Context, Result};
use dashmap::DashMap;
use shared::models::{ServiceProvider, SourceType, SyncRequest};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::client::PaperlessClient;
use crate::config::PaperlessConfig;
use crate::models::{
    build_document_created_event, build_document_deleted_event, build_document_updated_event,
    generate_document_content, Lookups, PaperlessConnectorState,
};

pub struct SyncManager {
    sdk_client: shared::SdkClient,
    /// Maps `sync_run_id` → cancellation flag.
    active_syncs: DashMap<String, Arc<AtomicBool>>,
}

impl SyncManager {
    pub fn new(sdk_client: shared::SdkClient) -> Self {
        Self {
            sdk_client,
            active_syncs: DashMap::new(),
        }
    }

    /// Signal the active sync for `sync_run_id` to stop.
    /// Returns `true` if a running sync was found and cancelled.
    pub fn cancel_sync(&self, sync_run_id: &str) -> bool {
        if let Some(flag) = self.active_syncs.get(sync_run_id) {
            flag.store(true, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Entry point for a sync run.  Must not panic; errors are reported via
    /// the SDK `fail` endpoint.
    pub async fn sync_source(&self, request: SyncRequest) -> Result<()> {
        let sync_run_id = &request.sync_run_id;

        // Install cancellation flag.
        let cancelled = Arc::new(AtomicBool::new(false));
        self.active_syncs.insert(sync_run_id.clone(), Arc::clone(&cancelled));

        let result = self.run_sync(&request, &cancelled).await;

        self.active_syncs.remove(sync_run_id);

        match result {
            Ok(()) => Ok(()),
            Err(e) => {
                error!("Paperless sync {} failed: {:#}", sync_run_id, e);
                if let Err(fe) = self.sdk_client.fail(sync_run_id, &e.to_string()).await {
                    warn!("Failed to report sync failure: {}", fe);
                }
                Err(e)
            }
        }
    }

    async fn run_sync(&self, request: &SyncRequest, cancelled: &AtomicBool) -> Result<()> {
        let sync_run_id = &request.sync_run_id;
        let source_id = &request.source_id;

        // ── Load source & credentials ──────────────────────────────────────
        let source = self
            .sdk_client
            .get_source(source_id)
            .await
            .context("Failed to fetch source")?;

        if source.source_type != SourceType::PaperlessNgx {
            anyhow::bail!("Source {} is not a Paperless-ngx source", source_id);
        }

        let config = PaperlessConfig::from_source_config(&source.config)?;

        let credentials = self
            .sdk_client
            .get_credentials(source_id)
            .await
            .context("Failed to fetch credentials")?;

        if credentials.provider != ServiceProvider::PaperlessNgx {
            anyhow::bail!("Unexpected credential provider for source {}", source_id);
        }

        let api_key = credentials
            .credentials
            .get("api_key")
            .and_then(|v| v.as_str())
            .context("Missing 'api_key' in credentials")?
            .to_string();

        // Resolve user email for permission assignment.
        let user_email = credentials.principal_email.as_deref().map(str::to_string);

        // ── Build API client ───────────────────────────────────────────────
        let client = PaperlessClient::new(config.base_url(), &api_key)
            .context("Failed to build Paperless client")?;

        // ── Load persistent state ──────────────────────────────────────────
        let mut state = PaperlessConnectorState::from_connector_state(&source.connector_state);

        // ── Fetch lookup tables (correspondents, types, tags, paths) ───────
        info!("Fetching Paperless-ngx metadata for source {}", source_id);
        let (correspondents, document_types, tags, storage_paths) = tokio::try_join!(
            client.fetch_correspondents(),
            client.fetch_document_types(),
            client.fetch_tags(),
            client.fetch_storage_paths(),
        )
        .context("Failed to fetch Paperless-ngx lookup tables")?;

        let lookups = Lookups {
            correspondents,
            document_types,
            tags,
            storage_paths,
        };

        // ── Paginate through all documents ─────────────────────────────────
        info!("Starting document sync for Paperless-ngx source {}", source_id);

        let mut scanned = 0i32;
        let mut processed = 0i32;
        let mut current_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();

        let mut next_url = Some(client.documents_first_page_url());
        while let Some(url) = next_url {
            if cancelled.load(Ordering::Relaxed) {
                info!("Paperless sync {} cancelled", sync_run_id);
                self.sdk_client.cancel(sync_run_id).await.ok();
                return Ok(());
            }

            let (docs, next) = client
                .fetch_documents_page(&url)
                .await
                .context("Failed to fetch documents page")?;
            next_url = next;

            for doc in docs {
                scanned += 1;
                let doc_id = doc.id;
                current_ids.insert(doc_id);

                let modified = doc.modified.clone().unwrap_or_default();
                let is_new = !state.indexed_ids.contains(&doc_id);
                let is_changed = !is_new
                    && state.modified_at.get(&doc_id).map_or(true, |m| m != &modified);

                if !is_new && !is_changed {
                    continue; // Already up-to-date
                }

                let resolved = lookups.resolve(&doc);
                let content = generate_document_content(&resolved);

                let content_id = match self
                    .sdk_client
                    .store_content(sync_run_id, &content)
                    .await
                    .context("Failed to store content")
                {
                    Ok(id) => id,
                    Err(e) => {
                        error!("Failed to store content for doc {}: {}", doc_id, e);
                        continue;
                    }
                };

                let doc_url = client.document_url(doc_id);
                let event = if is_new {
                    build_document_created_event(
                        &resolved,
                        sync_run_id.clone(),
                        source_id.clone(),
                        content_id,
                        doc_url,
                        user_email.as_deref(),
                    )
                } else {
                    build_document_updated_event(
                        &resolved,
                        sync_run_id.clone(),
                        source_id.clone(),
                        content_id,
                        doc_url,
                        user_email.as_deref(),
                    )
                };

                if let Err(e) = self.sdk_client.emit_event(sync_run_id, source_id, event).await {
                    error!("Failed to emit event for doc {}: {}", doc_id, e);
                    continue;
                }

                // Persist successful indexing.
                state.indexed_ids.insert(doc_id);
                state.modified_at.insert(doc_id, modified);
                processed += 1;
            }
        }

        // ── Deletion detection ─────────────────────────────────────────────
        let deleted_ids: Vec<i64> = state
            .indexed_ids
            .iter()
            .copied()
            .filter(|id| !current_ids.contains(id))
            .collect();

        for doc_id in deleted_ids {
            if cancelled.load(Ordering::Relaxed) {
                break;
            }
            let event = build_document_deleted_event(
                sync_run_id.clone(),
                source_id.clone(),
                doc_id,
            );
            if let Err(e) = self.sdk_client.emit_event(sync_run_id, source_id, event).await {
                error!("Failed to emit delete event for doc {}: {}", doc_id, e);
            } else {
                state.indexed_ids.remove(&doc_id);
                state.modified_at.remove(&doc_id);
                info!("Deleted document {} from index", doc_id);
            }
        }

        // ── Complete sync ──────────────────────────────────────────────────
        self.sdk_client
            .complete(
                sync_run_id,
                scanned,
                processed,
                Some(state.to_json()),
            )
            .await
            .context("Failed to complete sync")?;

        info!(
            "Paperless sync {} complete: scanned={}, processed={}",
            sync_run_id, scanned, processed
        );

        Ok(())
    }
}
