use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use redis::{AsyncCommands, Client as RedisClient};
use shared::models::{ServiceCredentials, ServiceProvider, SourceType, SyncRequest};
use std::collections::HashSet;
use tracing::{debug, error, info};

use crate::auth::{AtlassianCredentials, AuthManager};
use crate::confluence::ConfluenceProcessor;
use crate::jira::JiraProcessor;
use shared::SdkClient;

pub struct SyncManager {
    sdk_client: SdkClient,
    auth_manager: AuthManager,
    confluence_processor: ConfluenceProcessor,
    jira_processor: JiraProcessor,
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

    pub fn get_confluence_page_sync_key(
        &self,
        source_id: &str,
        space_id: &str,
        page_id: &str,
    ) -> String {
        if cfg!(test) {
            format!(
                "atlassian:confluence:page:test:{}:{}:{}",
                source_id, space_id, page_id
            )
        } else {
            format!(
                "atlassian:confluence:page:{}:{}:{}",
                source_id, space_id, page_id
            )
        }
    }

    pub async fn get_confluence_page_version(
        &self,
        source_id: &str,
        space_id: &str,
        page_id: &str,
    ) -> Result<Option<i32>> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let key = self.get_confluence_page_sync_key(source_id, space_id, page_id);

        let result: Option<String> = conn.get(&key).await?;
        if let Some(version_str) = result {
            if let Ok(version) = version_str.parse::<i32>() {
                return Ok(Some(version));
            }
        }
        Ok(None)
    }

    pub async fn set_confluence_page_version(
        &self,
        source_id: &str,
        space_id: &str,
        page_id: &str,
        version: i32,
    ) -> Result<()> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let key = self.get_confluence_page_sync_key(source_id, space_id, page_id);

        let _: () = conn.set_ex(&key, version, 30 * 24 * 60 * 60).await?; // 30 days expiry
        Ok(())
    }
}

impl SyncManager {
    pub fn new(redis_client: RedisClient, sdk_client: SdkClient) -> Self {
        Self {
            sdk_client: sdk_client.clone(),
            auth_manager: AuthManager::new(),
            confluence_processor: ConfluenceProcessor::new(
                sdk_client.clone(),
                redis_client.clone(),
            ),
            jira_processor: JiraProcessor::new(sdk_client),
        }
    }

    /// Execute a sync based on the request from connector-manager
    pub async fn sync_source(&mut self, request: SyncRequest) -> Result<()> {
        let sync_run_id = &request.sync_run_id;
        let source_id = &request.source_id;

        info!(
            "Starting sync for source: {} (sync_run_id: {})",
            source_id, sync_run_id
        );

        // Fetch source via SDK
        let source = self
            .sdk_client
            .get_source(source_id)
            .await
            .context("Failed to fetch source via SDK")?;

        if !source.is_active {
            let err_msg = format!("Source is not active: {}", source_id);
            self.sdk_client.fail(sync_run_id, &err_msg).await?;
            return Err(anyhow::anyhow!(err_msg));
        }

        let source_type = source.source_type.clone();
        if source_type != SourceType::Confluence && source_type != SourceType::Jira {
            let err_msg = format!(
                "Invalid source type for Atlassian connector: {:?}",
                source_type
            );
            self.sdk_client.fail(sync_run_id, &err_msg).await?;
            return Err(anyhow::anyhow!(err_msg));
        }

        // Fetch and validate credentials
        let service_creds = self.get_service_credentials(source_id).await?;
        let (base_url, user_email, api_token) =
            self.extract_atlassian_credentials(&service_creds)?;

        debug!("Validating Atlassian credentials...");
        let mut credentials = match self
            .get_or_validate_credentials(&base_url, &user_email, &api_token)
            .await
        {
            Ok(creds) => creds,
            Err(e) => {
                self.sdk_client.fail(sync_run_id, &e.to_string()).await?;
                return Err(e);
            }
        };
        debug!("Successfully validated Atlassian credentials.");

        if let Err(e) = self
            .auth_manager
            .ensure_valid_credentials(&mut credentials)
            .await
        {
            self.sdk_client.fail(sync_run_id, &e.to_string()).await?;
            return Err(e);
        }

        // Determine sync strategy based on sync_mode or last sync time
        let sync_start = Utc::now();
        let is_full_sync = request.sync_mode == "full" || source.last_sync_at.is_none();

        let result = if is_full_sync {
            info!("Performing full sync for source: {}", source.name);
            self.execute_full_sync(&credentials, source_id, sync_run_id, &source.source_type)
                .await
        } else {
            info!("Performing incremental sync for source: {}", source.name);
            let last_sync = source
                .last_sync_at
                .and_then(|last| DateTime::from_timestamp(last.unix_timestamp(), 0))
                .unwrap_or_else(|| sync_start - chrono::Duration::hours(24));

            self.execute_incremental_sync(
                &credentials,
                source_id,
                sync_run_id,
                &source.source_type,
                last_sync,
            )
            .await
        };

        match result {
            Ok(total_processed) => {
                info!(
                    "Sync completed for source {}: {} documents processed",
                    source.name, total_processed
                );
                self.sdk_client
                    .complete(
                        sync_run_id,
                        total_processed as i32,
                        total_processed as i32,
                        None,
                    )
                    .await?;
                Ok(())
            }
            Err(e) => {
                error!("Sync failed for source {}: {}", source.name, e);
                self.sdk_client.fail(sync_run_id, &e.to_string()).await?;
                Err(e)
            }
        }
    }

    async fn execute_full_sync(
        &mut self,
        credentials: &AtlassianCredentials,
        source_id: &str,
        sync_run_id: &str,
        source_type: &SourceType,
    ) -> Result<u32> {
        match source_type {
            SourceType::Confluence => {
                self.confluence_processor
                    .sync_all_spaces(credentials, source_id, sync_run_id)
                    .await
            }
            SourceType::Jira => {
                self.jira_processor
                    .sync_all_projects(credentials, source_id, sync_run_id)
                    .await
            }
            _ => Err(anyhow!("Unsupported source type: {:?}", source_type)),
        }
    }

    async fn execute_incremental_sync(
        &mut self,
        credentials: &AtlassianCredentials,
        source_id: &str,
        sync_run_id: &str,
        source_type: &SourceType,
        last_sync: DateTime<Utc>,
    ) -> Result<u32> {
        match source_type {
            SourceType::Confluence => {
                self.confluence_processor
                    .sync_all_spaces_incremental(credentials, source_id, sync_run_id)
                    .await
            }
            SourceType::Jira => {
                self.jira_processor
                    .sync_issues_updated_since(credentials, source_id, last_sync, None, sync_run_id)
                    .await
            }
            _ => Err(anyhow!("Unsupported source type: {:?}", source_type)),
        }
    }

    async fn get_service_credentials(&self, source_id: &str) -> Result<ServiceCredentials> {
        let creds = self
            .sdk_client
            .get_credentials(source_id)
            .await
            .context("Failed to fetch credentials via SDK")?;

        if creds.provider != ServiceProvider::Atlassian {
            return Err(anyhow::anyhow!(
                "Expected Atlassian credentials for source {}, found {:?}",
                source_id,
                creds.provider
            ));
        }

        Ok(creds)
    }

    fn extract_atlassian_credentials(
        &self,
        creds: &ServiceCredentials,
    ) -> Result<(String, String, String)> {
        let base_url = creds
            .config
            .get("base_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing base_url in service credentials config"))?
            .to_string();

        let user_email = creds
            .principal_email
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Missing principal_email in service credentials"))?
            .to_string();

        let api_token = creds
            .credentials
            .get("api_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing api_token in service credentials"))?
            .to_string();

        Ok((base_url, user_email, api_token))
    }

    async fn get_or_validate_credentials(
        &self,
        base_url: &str,
        user_email: &str,
        api_token: &str,
    ) -> Result<AtlassianCredentials> {
        self.auth_manager
            .validate_credentials(base_url, user_email, api_token)
            .await
    }

    pub async fn test_connection(
        &self,
        config: &(String, String, String),
    ) -> Result<(Vec<String>, Vec<String>)> {
        let (base_url, user_email, api_token) = config;
        let credentials = self
            .get_or_validate_credentials(base_url, user_email, api_token)
            .await?;

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
}
