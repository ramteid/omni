use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedSyncRequest {
    pub sync_run_id: String,
    pub source_id: String,
    pub sync_mode: String,
    #[serde(default)]
    pub last_sync_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedCancelRequest {
    pub sync_run_id: String,
}

#[derive(Clone)]
struct MockState {
    sync_requests: Arc<Mutex<Vec<RecordedSyncRequest>>>,
    cancel_requests: Arc<Mutex<Vec<RecordedCancelRequest>>>,
    sync_response_status: Arc<Mutex<StatusCode>>,
    sync_response_body: Arc<Mutex<JsonValue>>,
}

pub struct MockConnector {
    pub base_url: String,
    pub sync_requests: Arc<Mutex<Vec<RecordedSyncRequest>>>,
    pub cancel_requests: Arc<Mutex<Vec<RecordedCancelRequest>>>,
    sync_response_status: Arc<Mutex<StatusCode>>,
    sync_response_body: Arc<Mutex<JsonValue>>,
    _server_handle: tokio::task::JoinHandle<()>,
}

impl MockConnector {
    pub async fn start() -> anyhow::Result<Self> {
        let sync_requests: Arc<Mutex<Vec<RecordedSyncRequest>>> = Arc::new(Mutex::new(Vec::new()));
        let cancel_requests: Arc<Mutex<Vec<RecordedCancelRequest>>> =
            Arc::new(Mutex::new(Vec::new()));
        let sync_response_status = Arc::new(Mutex::new(StatusCode::OK));
        let sync_response_body = Arc::new(Mutex::new(json!({"status": "accepted"})));

        let state = MockState {
            sync_requests: sync_requests.clone(),
            cancel_requests: cancel_requests.clone(),
            sync_response_status: sync_response_status.clone(),
            sync_response_body: sync_response_body.clone(),
        };

        let app = Router::new()
            .route("/health", get(health))
            .route(
                "/manifest",
                get(|| async {
                    Json(json!({
                        "name": "mock-connector",
                        "version": "1.0.0",
                        "sync_modes": ["full", "incremental"],
                        "actions": []
                    }))
                }),
            )
            .route("/sync", post(handle_sync))
            .route("/cancel", post(handle_cancel))
            .with_state(state);

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();

        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        sleep(Duration::from_millis(50)).await;

        Ok(Self {
            base_url: format!("http://127.0.0.1:{}", port),
            sync_requests,
            cancel_requests,
            sync_response_status,
            sync_response_body,
            _server_handle: server_handle,
        })
    }

    pub fn set_sync_response(&self, status: StatusCode, body: JsonValue) {
        *self.sync_response_status.lock().unwrap() = status;
        *self.sync_response_body.lock().unwrap() = body;
    }

    pub fn get_sync_requests(&self) -> Vec<RecordedSyncRequest> {
        self.sync_requests.lock().unwrap().clone()
    }

    pub fn get_cancel_requests(&self) -> Vec<RecordedCancelRequest> {
        self.cancel_requests.lock().unwrap().clone()
    }
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn handle_sync(
    State(state): State<MockState>,
    Json(request): Json<RecordedSyncRequest>,
) -> (StatusCode, Json<JsonValue>) {
    state.sync_requests.lock().unwrap().push(request);
    let status = *state.sync_response_status.lock().unwrap();
    let body = state.sync_response_body.lock().unwrap().clone();
    (status, Json(body))
}

async fn handle_cancel(
    State(state): State<MockState>,
    Json(request): Json<RecordedCancelRequest>,
) -> StatusCode {
    state.cancel_requests.lock().unwrap().push(request);
    StatusCode::OK
}
