use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::sync::SyncManager;

#[derive(Clone)]
pub struct ApiState {
    pub sync_manager: Arc<Mutex<SyncManager>>,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub service: String,
}

#[derive(Serialize)]
pub struct SyncResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Deserialize)]
pub struct TestConnectionRequest {
    pub base_url: String,
    pub user_email: String,
    pub api_token: String,
}

#[derive(Serialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
    pub jira_projects: Vec<String>,
    pub confluence_spaces: Vec<String>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn create_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/sync/:source_id", post(trigger_sync))
        .route("/sync", post(trigger_full_sync))
        .route("/test-connection", post(test_connection))
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        service: "atlassian-connector".to_string(),
    })
}

async fn trigger_sync(
    State(state): State<ApiState>,
    Path(source_id): Path<String>,
) -> Json<SyncResponse> {
    info!("Received sync request for source: {}", source_id);

    let sync_manager = state.sync_manager.clone();
    let source_id_clone = source_id.clone();

    tokio::spawn(async move {
        let mut manager = sync_manager.lock().await;
        match manager.sync_source_by_id(source_id_clone.clone()).await {
            Ok(_) => {
                info!(
                    "Successfully completed sync for source: {}",
                    source_id_clone
                );
            }
            Err(e) => {
                error!(
                    "Failed to complete sync for source {}: {}",
                    source_id_clone, e
                );
            }
        }
    });

    Json(SyncResponse {
        success: true,
        message: format!(
            "Sync triggered successfully for source: {}. Running in background.",
            source_id
        ),
    })
}

async fn trigger_full_sync(State(state): State<ApiState>) -> Json<SyncResponse> {
    info!("Manual sync triggered via API");

    let sync_manager = state.sync_manager.clone();

    tokio::spawn(async move {
        let mut manager = sync_manager.lock().await;
        match manager.sync_all_sources().await {
            Ok(()) => {
                info!("Manual sync completed successfully");
            }
            Err(e) => {
                error!("Manual sync failed: {}", e);
            }
        }
    });

    Json(SyncResponse {
        success: true,
        message: "Sync triggered successfully. Running in background.".to_string(),
    })
}

async fn test_connection(
    State(state): State<ApiState>,
    Json(request): Json<TestConnectionRequest>,
) -> Result<Json<TestConnectionResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Testing connection to Atlassian: {}", request.base_url);

    let config = (request.base_url, request.user_email, request.api_token);

    let sync_manager = state.sync_manager.lock().await;
    match sync_manager.test_connection(&config).await {
        Ok((jira_projects, confluence_spaces)) => {
            info!(
                "Connection test successful: {} JIRA projects, {} Confluence spaces",
                jira_projects.len(),
                confluence_spaces.len()
            );
            Ok(Json(TestConnectionResponse {
                success: true,
                message: format!(
                    "Successfully connected. Found {} JIRA projects and {} Confluence spaces.",
                    jira_projects.len(),
                    confluence_spaces.len()
                ),
                jira_projects,
                confluence_spaces,
            }))
        }
        Err(e) => {
            error!("Connection test failed: {}", e);
            Ok(Json(TestConnectionResponse {
                success: false,
                message: format!("Connection failed: {}", e),
                jira_projects: vec![],
                confluence_spaces: vec![],
            }))
        }
    }
}
