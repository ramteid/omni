use axum::{
    extract::{Path, State},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{error, info};

use crate::sync::SyncManager;

#[derive(Clone)]
pub struct ApiState {
    pub sync_manager: Arc<SyncManager>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncResponse {
    pub success: bool,
    pub message: String,
}

pub fn create_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/sync/:source_id", post(trigger_sync))
        .with_state(state)
}

async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "service": "clio-google-connector"
    }))
}

async fn trigger_sync(
    State(state): State<ApiState>,
    Path(source_id): Path<String>,
) -> Json<SyncResponse> {
    info!("Received sync request for source: {}", source_id);

    let sync_manager = state.sync_manager.clone();
    let source_id_clone = source_id.clone();
    
    tokio::spawn(async move {
        match sync_manager.sync_source_by_id(source_id_clone.clone()).await {
            Ok(_) => {
                info!("Successfully completed sync for source: {}", source_id_clone);
            }
            Err(e) => {
                error!("Failed to complete sync for source {}: {}", source_id_clone, e);
            }
        }
    });

    Json(SyncResponse {
        success: true,
        message: format!("Sync triggered successfully for source: {}. Running in background.", source_id),
    })
}