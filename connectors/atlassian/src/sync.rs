use anyhow::Result;
use chrono::{DateTime, Utc};
use redis::{AsyncCommands, Client as RedisClient};
use shared::models::{Source, SourceType};
use shared::queue::EventQueue;
use shared::ContentStorage;
use sqlx::{PgPool, Row};
use std::collections::HashSet;
use tracing::{debug, error, info, warn};

use crate::auth::{AtlassianCredentials, AuthManager};
use crate::confluence::ConfluenceProcessor;
use crate::jira::JiraProcessor;

pub struct SyncManager {
    pool: PgPool,
    redis_client: RedisClient,
    auth_manager: AuthManager,
    confluence_processor: ConfluenceProcessor,
    jira_processor: JiraProcessor,
    event_queue: EventQueue,
}

pub struct SyncState {
    redis_client: RedisClient,
}

impl SyncState {
    pub fn new(redis_client: RedisClient) -> Self {
        Self { redis_client }
    }

    pub fn get_confluence_sync_key(&self, source_id: &str, space_key: &str) -> String {
        format!("atlassian:confluence:sync:{}:{}", source_id, space_key)
    }

    pub fn get_jira_sync_key(&self, source_id: &str, project_key: &str) -> String {
        format!("atlassian:jira:sync:{}:{}", source_id, project_key)
    }

    pub fn get_test_confluence_sync_key(&self, source_id: &str, space_key: &str) -> String {
        format!("atlassian:confluence:sync:test:{}:{}", source_id, space_key)
    }

    pub fn get_test_jira_sync_key(&self, source_id: &str, project_key: &str) -> String {
        format!("atlassian:jira:sync:test:{}:{}", source_id, project_key)
    }

    pub async fn get_confluence_last_sync(
        &self,
        source_id: &str,
        space_key: &str,
    ) -> Result<Option<DateTime<Utc>>> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let key = if cfg!(test) {
            self.get_test_confluence_sync_key(source_id, space_key)
        } else {
            self.get_confluence_sync_key(source_id, space_key)
        };

        let result: Option<String> = conn.get(&key).await?;
        if let Some(timestamp_str) = result {
            if let Ok(timestamp) = timestamp_str.parse::<i64>() {
                if let Some(dt) = DateTime::from_timestamp(timestamp, 0) {
                    return Ok(Some(dt));
                }
            }
        }
        Ok(None)
    }

    pub async fn set_confluence_last_sync(
        &self,
        source_id: &str,
        space_key: &str,
        sync_time: DateTime<Utc>,
    ) -> Result<()> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let key = if cfg!(test) {
            self.get_test_confluence_sync_key(source_id, space_key)
        } else {
            self.get_confluence_sync_key(source_id, space_key)
        };

        let timestamp = sync_time.timestamp();
        let _: () = conn.set_ex(&key, timestamp, 30 * 24 * 60 * 60).await?; // 30 days expiry
        Ok(())
    }

    pub async fn get_jira_last_sync(
        &self,
        source_id: &str,
        project_key: &str,
    ) -> Result<Option<DateTime<Utc>>> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let key = if cfg!(test) {
            self.get_test_jira_sync_key(source_id, project_key)
        } else {
            self.get_jira_sync_key(source_id, project_key)
        };

        let result: Option<String> = conn.get(&key).await?;
        if let Some(timestamp_str) = result {
            if let Ok(timestamp) = timestamp_str.parse::<i64>() {
                if let Some(dt) = DateTime::from_timestamp(timestamp, 0) {
                    return Ok(Some(dt));
                }
            }
        }
        Ok(None)
    }

    pub async fn set_jira_last_sync(
        &self,
        source_id: &str,
        project_key: &str,
        sync_time: DateTime<Utc>,
    ) -> Result<()> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let key = if cfg!(test) {
            self.get_test_jira_sync_key(source_id, project_key)
        } else {
            self.get_jira_sync_key(source_id, project_key)
        };

        let timestamp = sync_time.timestamp();
        let _: () = conn.set_ex(&key, timestamp, 30 * 24 * 60 * 60).await?; // 30 days expiry
        Ok(())
    }

    pub async fn get_all_synced_confluence_spaces(
        &self,
        source_id: &str,
    ) -> Result<HashSet<String>> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let pattern = if cfg!(test) {
            format!("atlassian:confluence:sync:test:{}:*", source_id)
        } else {
            format!("atlassian:confluence:sync:{}:*", source_id)
        };

        let keys: Vec<String> = conn.keys(&pattern).await?;
        let prefix = if cfg!(test) {
            format!("atlassian:confluence:sync:test:{}:", source_id)
        } else {
            format!("atlassian:confluence:sync:{}:", source_id)
        };

        let space_keys: HashSet<String> = keys
            .into_iter()
            .filter_map(|key| key.strip_prefix(&prefix).map(|s| s.to_string()))
            .collect();

        Ok(space_keys)
    }

    pub async fn get_all_synced_jira_projects(&self, source_id: &str) -> Result<HashSet<String>> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let pattern = if cfg!(test) {
            format!("atlassian:jira:sync:test:{}:*", source_id)
        } else {
            format!("atlassian:jira:sync:{}:*", source_id)
        };

        let keys: Vec<String> = conn.keys(&pattern).await?;
        let prefix = if cfg!(test) {
            format!("atlassian:jira:sync:test:{}:", source_id)
        } else {
            format!("atlassian:jira:sync:{}:", source_id)
        };

        let project_keys: HashSet<String> = keys
            .into_iter()
            .filter_map(|key| key.strip_prefix(&prefix).map(|s| s.to_string()))
            .collect();

        Ok(project_keys)
    }
}

impl SyncManager {
    pub async fn new(pool: PgPool, redis_client: RedisClient) -> Result<Self> {
        let event_queue = EventQueue::new(pool.clone());
        let content_storage = ContentStorage::new(pool.clone());

        Ok(Self {
            pool,
            redis_client,
            auth_manager: AuthManager::new(),
            confluence_processor: ConfluenceProcessor::new(
                event_queue.clone(),
                content_storage.clone(),
            ),
            jira_processor: JiraProcessor::new(event_queue.clone(), content_storage.clone()),
            event_queue,
        })
    }

    pub async fn sync_all_sources(&mut self) -> Result<()> {
        let sources = self.get_active_sources().await?;

        info!("Found {} active Atlassian sources", sources.len());

        for source in sources {
            if let Err(e) = self.sync_source(&source).await {
                error!("Failed to sync source {}: {}", source.id, e);
                self.update_source_status(&source.id, "failed").await?;
            }
        }

        Ok(())
    }

    async fn sync_source(&mut self, source: &Source) -> Result<()> {
        info!("Starting sync for Atlassian source: {}", source.name);

        let config = self.get_source_config(&source.id).await?;
        let mut credentials = self.get_or_validate_credentials(&config).await?;

        self.auth_manager
            .ensure_valid_credentials(&mut credentials)
            .await?;

        let sync_start = Utc::now();
        self.update_source_status(&source.id, "syncing").await?;

        // Determine sync strategy based on last sync time
        let should_do_full_sync = source.last_sync_at.is_none()
            || source
                .last_sync_at
                .map(|last| {
                    let last_utc =
                        DateTime::from_timestamp(last.unix_timestamp(), 0).unwrap_or_default();
                    (sync_start - last_utc).num_hours() > 24
                })
                .unwrap_or(true);

        let mut total_processed = 0;

        if should_do_full_sync {
            info!("Performing full sync for source: {}", source.name);

            // Full sync for Confluence
            match self
                .confluence_processor
                .sync_all_spaces(&credentials, &source.id)
                .await
            {
                Ok(pages_count) => {
                    total_processed += pages_count;
                    info!("Full Confluence sync completed: {} pages", pages_count);
                }
                Err(e) => {
                    error!("Full Confluence sync failed: {}", e);
                }
            }

            // Full sync for JIRA
            match self
                .jira_processor
                .sync_all_projects(&credentials, &source.id)
                .await
            {
                Ok(issues_count) => {
                    total_processed += issues_count;
                    info!("Full JIRA sync completed: {} issues", issues_count);
                }
                Err(e) => {
                    error!("Full JIRA sync failed: {}", e);
                }
            }
        } else {
            info!("Performing incremental sync for source: {}", source.name);

            let since = source
                .last_sync_at
                .map(|dt| DateTime::from_timestamp(dt.unix_timestamp(), 0).unwrap_or_default())
                .unwrap_or_else(|| sync_start - chrono::Duration::days(1));

            // Incremental sync for Confluence
            match self
                .confluence_processor
                .sync_pages_updated_since(&credentials, &source.id, since)
                .await
            {
                Ok(pages_count) => {
                    total_processed += pages_count;
                    info!(
                        "Incremental Confluence sync completed: {} pages",
                        pages_count
                    );
                }
                Err(e) => {
                    error!("Incremental Confluence sync failed: {}", e);
                }
            }

            // Incremental sync for JIRA
            match self
                .jira_processor
                .sync_issues_updated_since(&credentials, &source.id, since, None)
                .await
            {
                Ok(issues_count) => {
                    total_processed += issues_count;
                    info!("Incremental JIRA sync completed: {} issues", issues_count);
                }
                Err(e) => {
                    error!("Incremental JIRA sync failed: {}", e);
                }
            }
        }

        // Update source status
        if total_processed > 0 {
            self.update_source_success(&source.id, total_processed, sync_start)
                .await?;
            info!(
                "Successfully synced {} documents from source: {}",
                total_processed, source.name
            );
        } else {
            self.update_source_status(&source.id, "completed").await?;
            info!(
                "Sync completed with no new documents for source: {}",
                source.name
            );
        }

        Ok(())
    }

    async fn get_active_sources(&self) -> Result<Vec<Source>> {
        let sources = sqlx::query_as::<_, Source>(
            "SELECT s.* FROM sources s
             WHERE (s.source_type = $1 OR s.source_type = $2)
             AND s.is_active = true",
        )
        .bind(SourceType::Confluence)
        .bind(SourceType::Jira)
        .fetch_all(&self.pool)
        .await?;

        Ok(sources)
    }

    async fn get_source_config(&self, source_id: &str) -> Result<(String, String, String)> {
        let row = sqlx::query("SELECT config FROM sources WHERE id = $1")
            .bind(source_id)
            .fetch_one(&self.pool)
            .await?;

        let config: serde_json::Value = row.try_get("config")?;

        let base_url = config
            .get("base_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing base_url in source config"))?
            .to_string();

        let user_email = config
            .get("user_email")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing user_email in source config"))?
            .to_string();

        let api_token = config
            .get("api_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing api_token in source config"))?
            .to_string();

        Ok((base_url, user_email, api_token))
    }

    async fn get_or_validate_credentials(
        &self,
        config: &(String, String, String),
    ) -> Result<AtlassianCredentials> {
        let (base_url, user_email, api_token) = config;

        // Always validate credentials to ensure they're working
        self.auth_manager
            .validate_credentials(base_url, user_email, api_token)
            .await
    }

    async fn update_source_status(&self, source_id: &str, status: &str) -> Result<()> {
        sqlx::query("UPDATE sources SET sync_status = $1, updated_at = NOW() WHERE id = $2")
            .bind(status)
            .bind(source_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_source_success(
        &self,
        source_id: &str,
        document_count: u32,
        sync_time: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE sources SET 
             sync_status = 'completed', 
             last_sync_at = $1, 
             sync_error = NULL,
             updated_at = NOW() 
             WHERE id = $2",
        )
        .bind(sync_time)
        .bind(source_id)
        .execute(&self.pool)
        .await?;

        info!(
            "Updated source {} with {} documents at {}",
            source_id, document_count, sync_time
        );
        Ok(())
    }

    pub async fn test_connection(
        &self,
        config: &(String, String, String),
    ) -> Result<(Vec<String>, Vec<String>)> {
        let credentials = self.get_or_validate_credentials(config).await?;

        let jira_projects = self
            .auth_manager
            .test_jira_permissions(&credentials)
            .await?;
        let confluence_spaces = self
            .auth_manager
            .test_confluence_permissions(&credentials)
            .await?;

        Ok((jira_projects, confluence_spaces))
    }

    pub fn get_sync_state(&self) -> SyncState {
        SyncState::new(self.redis_client.clone())
    }
}
