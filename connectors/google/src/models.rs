use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub source_type: String,
    pub config: serde_json::Value,
    pub oauth_credentials: Option<serde_json::Value>,
    pub is_active: bool,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub sync_status: Option<String>,
    pub sync_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleDriveFile {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub web_view_link: Option<String>,
    pub created_time: Option<String>,
    pub modified_time: Option<String>,
    pub size: Option<String>,
    pub parents: Option<Vec<String>>,
    pub shared: Option<bool>,
    pub permissions: Option<Vec<Permission>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Permission {
    pub id: String,
    pub r#type: String,
    pub email_address: Option<String>,
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct DocumentEvent {
    pub source_id: String,
    pub document_id: String,
    pub title: String,
    pub content: String,
    pub url: Option<String>,
    pub metadata: serde_json::Value,
    pub permissions: Vec<String>,
}

impl DocumentEvent {
    pub fn from_drive_file(source_id: String, file: &GoogleDriveFile, content: String) -> Self {
        let mut permissions = Vec::new();

        if let Some(file_permissions) = &file.permissions {
            for perm in file_permissions {
                if let Some(email) = &perm.email_address {
                    permissions.push(email.clone());
                }
            }
        }

        let metadata = serde_json::json!({
            "file_id": file.id,
            "mime_type": file.mime_type,
            "size": file.size,
            "created_time": file.created_time,
            "modified_time": file.modified_time,
            "shared": file.shared.unwrap_or(false),
        });

        Self {
            source_id,
            document_id: file.id.clone(),
            title: file.name.clone(),
            content,
            url: file.web_view_link.clone(),
            metadata,
            permissions,
        }
    }
}
