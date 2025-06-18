use axum::{
    extract::State,
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
    pub message: String,
    pub sources_synced: usize,
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
        .route("/sync", post(trigger_sync))
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
) -> Result<Json<SyncResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Manual sync triggered via API");

    let mut sync_manager = state.sync_manager.lock().await;
    match sync_manager.sync_all_sources().await {
        Ok(()) => {
            info!("Manual sync completed successfully");
            Ok(Json(SyncResponse {
                message: "Sync completed successfully".to_string(),
                sources_synced: 0, // TODO: Return actual count
            }))
        }
        Err(e) => {
            error!("Manual sync failed: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Sync failed: {}", e),
                }),
            ))
        }
    }
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
