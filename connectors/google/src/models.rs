use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared::models::{ConnectorEvent, DocumentMetadata, DocumentPermissions};
use sqlx::types::time::OffsetDateTime;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

use crate::gmail::GmailMessage;

#[derive(Debug, Clone)]
pub struct UserFile {
    pub user_email: Arc<String>,
    pub file: GoogleDriveFile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleDriveFile {
    pub id: String,
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "webViewLink")]
    pub web_view_link: Option<String>,
    #[serde(rename = "createdTime")]
    pub created_time: Option<String>,
    #[serde(rename = "modifiedTime")]
    pub modified_time: Option<String>,
    pub size: Option<String>,
    pub parents: Option<Vec<String>>,
    pub shared: Option<bool>,
    pub permissions: Option<Vec<Permission>>,
    pub owners: Option<Vec<Owner>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Owner {
    pub id: String,
    #[serde(rename = "emailAddress")]
    pub email_address: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: String,
    #[serde(rename = "type")]
    pub permission_type: String,
    #[serde(rename = "emailAddress")]
    pub email_address: Option<String>,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct FolderMetadata {
    pub id: String,
    pub name: String,
    pub parents: Option<Vec<String>>,
}

impl From<GoogleDriveFile> for FolderMetadata {
    fn from(file: GoogleDriveFile) -> Self {
        Self {
            id: file.id,
            name: file.name,
            parents: file.parents,
        }
    }
}

impl GoogleDriveFile {
    pub fn to_connector_event(
        &self,
        sync_run_id: &str,
        source_id: &str,
        content_id: &str,
        path: Option<String>,
    ) -> ConnectorEvent {
        let mut users = Vec::new();

        if let Some(file_permissions) = &self.permissions {
            for perm in file_permissions {
                if let Some(email) = &perm.email_address {
                    users.push(email.clone());
                }
            }
        }

        let mut extra = HashMap::new();
        extra.insert("file_id".to_string(), serde_json::json!(self.id));
        extra.insert(
            "shared".to_string(),
            serde_json::json!(self.shared.unwrap_or(false)),
        );

        // Store Google Drive specific hierarchical data
        let mut google_drive_metadata = HashMap::new();
        if let Some(parents) = &self.parents {
            google_drive_metadata.insert("parents".to_string(), serde_json::json!(parents));
            if let Some(parent) = parents.first() {
                google_drive_metadata.insert("parent_id".to_string(), serde_json::json!(parent));
            }
        }
        extra.insert(
            "google_drive".to_string(),
            serde_json::json!(google_drive_metadata),
        );

        let metadata = DocumentMetadata {
            title: Some(self.name.clone()),
            author: None,
            created_at: self.created_time.as_ref().and_then(|t| {
                t.parse::<DateTime<Utc>>()
                    .ok()
                    .map(|dt| OffsetDateTime::from_unix_timestamp(dt.timestamp()).unwrap())
            }),
            updated_at: self.modified_time.as_ref().and_then(|t| {
                t.parse::<DateTime<Utc>>()
                    .ok()
                    .map(|dt| OffsetDateTime::from_unix_timestamp(dt.timestamp()).unwrap())
            }),
            mime_type: Some(self.mime_type.clone()),
            size: self.size.clone(),
            url: self.web_view_link.clone(),
            path,
            extra: Some(extra),
        };

        let permissions = DocumentPermissions {
            public: false,
            users,
            groups: vec![],
        };

        ConnectorEvent::DocumentCreated {
            sync_run_id: sync_run_id.to_string(),
            source_id: source_id.to_string(),
            document_id: self.id.clone(),
            content_id: content_id.to_string(),
            metadata,
            permissions,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookChannel {
    pub id: String,
    #[serde(rename = "type")]
    pub channel_type: String,
    pub address: String,
    pub params: Option<HashMap<String, String>>,
    pub expiration: Option<String>,
    pub token: Option<String>,
}

impl WebhookChannel {
    pub fn new(webhook_url: String, token: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            channel_type: "web_hook".to_string(),
            address: webhook_url,
            params: None,
            expiration: None,
            token,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookChannelResponse {
    pub id: String,
    #[serde(rename = "resourceId")]
    pub resource_id: String,
    #[serde(rename = "resourceUri")]
    pub resource_uri: String,
    pub token: Option<String>,
    pub expiration: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WebhookNotification {
    pub channel_id: String,
    pub resource_state: String,
    pub resource_id: Option<String>,
    pub resource_uri: Option<String>,
    pub changed: Option<String>,
}

impl WebhookNotification {
    pub fn from_headers(headers: &axum::http::HeaderMap) -> Option<Self> {
        let channel_id = headers.get("x-goog-channel-id")?.to_str().ok()?.to_string();

        let resource_state = headers
            .get("x-goog-resource-state")?
            .to_str()
            .ok()?
            .to_string();

        let resource_id = headers
            .get("x-goog-resource-id")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let resource_uri = headers
            .get("x-goog-resource-uri")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let changed = headers
            .get("x-goog-changed")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        Some(Self {
            channel_id,
            resource_state,
            resource_id,
            resource_uri,
            changed,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveChangesResponse {
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    pub changes: Vec<DriveChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveChange {
    #[serde(rename = "changeType")]
    pub change_type: String,
    pub removed: Option<bool>,
    pub file: Option<GoogleDriveFile>,
    #[serde(rename = "fileId")]
    pub file_id: Option<String>,
    pub time: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GooglePresentation {
    #[serde(rename = "presentationId")]
    pub presentation_id: String,
    pub title: String,
    pub slides: Vec<Slide>,
}

#[derive(Debug, Deserialize)]
pub struct Slide {
    #[serde(rename = "objectId")]
    pub object_id: String,
    #[serde(rename = "pageElements", default)]
    pub page_elements: Vec<PageElement>,
}

#[derive(Debug, Deserialize)]
pub struct PageElement {
    #[serde(rename = "objectId")]
    pub object_id: String,
    pub shape: Option<Shape>,
    pub table: Option<Table>,
}

#[derive(Debug, Deserialize)]
pub struct Shape {
    pub text: Option<TextContent>,
}

#[derive(Debug, Deserialize)]
pub struct Table {
    #[serde(rename = "tableRows", default)]
    pub table_rows: Vec<TableRow>,
}

#[derive(Debug, Deserialize)]
pub struct TableRow {
    #[serde(rename = "tableCells", default)]
    pub table_cells: Vec<TableCell>,
}

#[derive(Debug, Deserialize)]
pub struct TableCell {
    pub text: Option<TextContent>,
}

#[derive(Debug, Deserialize)]
pub struct TextContent {
    #[serde(rename = "textElements", default)]
    pub text_elements: Vec<TextElement>,
}

#[derive(Debug, Deserialize)]
pub struct TextElement {
    #[serde(rename = "textRun")]
    pub text_run: Option<TextRun>,
}

#[derive(Debug, Deserialize)]
pub struct TextRun {
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct GmailThread {
    pub thread_id: String,
    pub messages: Vec<GmailMessage>,
    pub participants: HashSet<String>,
    pub subject: String,
    pub latest_date: String,
    pub total_messages: usize,
}

impl GmailThread {
    pub fn new(thread_id: String) -> Self {
        Self {
            thread_id,
            messages: Vec::new(),
            participants: HashSet::new(),
            subject: String::new(),
            latest_date: String::new(),
            total_messages: 0,
        }
    }

    pub fn add_message(&mut self, message: GmailMessage) {
        // Update subject from first message if not set
        if self.subject.is_empty() {
            if let Some(subject) = self.extract_header_value(&message, "Subject") {
                self.subject = subject;
            }
        }

        // Extract participants from headers
        self.extract_participants(&message);

        // Update latest date
        if let Some(internal_date) = &message.internal_date {
            if self.latest_date.is_empty() {
                self.latest_date = internal_date.clone();
            } else {
                // Parse both dates as timestamps for proper comparison
                if let (Ok(current_ts), Ok(latest_ts)) = (
                    internal_date.parse::<i64>(),
                    self.latest_date.parse::<i64>(),
                ) {
                    if current_ts > latest_ts {
                        self.latest_date = internal_date.clone();
                    }
                }
            }
        }

        self.messages.push(message);
        self.total_messages = self.messages.len();
    }

    fn extract_participants(&mut self, message: &GmailMessage) {
        let headers_to_check = ["From", "To", "Cc", "Bcc"];

        for header_name in &headers_to_check {
            if let Some(header_value) = self.extract_header_value(message, header_name) {
                // Parse email addresses from header value
                // Simple parsing - in production might want more sophisticated email parsing
                for email in header_value.split(',') {
                    let email = email.trim();
                    // Extract email from "Name <email@domain.com>" format
                    if let Some(start) = email.find('<') {
                        if let Some(end) = email.find('>') {
                            if start < end {
                                let extracted_email = email[start + 1..end].trim().to_lowercase();
                                if !extracted_email.is_empty() {
                                    self.participants.insert(extracted_email);
                                }
                            }
                        }
                    } else if email.contains('@') {
                        // Direct email format
                        self.participants.insert(email.to_lowercase());
                    }
                }
            }
        }
    }

    fn extract_header_value(&self, message: &GmailMessage, header_name: &str) -> Option<String> {
        message
            .payload
            .as_ref()?
            .headers
            .as_ref()?
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case(header_name))
            .map(|h| h.value.clone())
    }

    pub fn aggregate_content(
        &self,
        gmail_client: &crate::gmail::GmailClient,
    ) -> Result<String, anyhow::Error> {
        let mut content_parts = Vec::new();

        // Add subject as the first part
        if !self.subject.is_empty() {
            content_parts.push(format!("Subject: {}", self.subject));
            content_parts.push(String::new()); // Empty line
        }

        // Add each message content
        for (i, message) in self.messages.iter().enumerate() {
            content_parts.push(format!("=== Message {} ===", i + 1));

            // Add basic message info
            if let Some(from) = self.extract_header_value(message, "From") {
                content_parts.push(format!("From: {}", from));
            }
            if let Some(date) = &message.internal_date {
                content_parts.push(format!("Date: {}", date));
            }

            content_parts.push(String::new()); // Empty line

            // Add message content
            match gmail_client.extract_message_content(message) {
                Ok(message_content) => {
                    if !message_content.trim().is_empty() {
                        content_parts.push(message_content.trim().to_string());
                    }
                }
                Err(e) => {
                    content_parts.push(format!("Error extracting message content: {}", e));
                }
            }

            content_parts.push(String::new()); // Empty line between messages
        }

        Ok(content_parts.join("\n"))
    }

    pub fn to_connector_event(
        &self,
        sync_run_id: &str,
        source_id: &str,
        content_id: &str,
        _gmail_client: &crate::gmail::GmailClient,
    ) -> Result<ConnectorEvent, anyhow::Error> {
        let mut extra = HashMap::new();
        extra.insert("thread_id".to_string(), serde_json::json!(self.thread_id));
        extra.insert(
            "message_count".to_string(),
            serde_json::json!(self.total_messages),
        );
        extra.insert(
            "participants".to_string(),
            serde_json::json!(self.participants.iter().collect::<Vec<_>>()),
        );

        // Parse latest date for metadata
        let updated_at = if !self.latest_date.is_empty() {
            self.latest_date
                .parse::<i64>()
                .ok()
                .and_then(|millis| OffsetDateTime::from_unix_timestamp(millis / 1000).ok())
        } else {
            None
        };

        let metadata = DocumentMetadata {
            title: Some(if self.subject.is_empty() {
                format!("Gmail Thread {}", self.thread_id)
            } else {
                self.subject.clone()
            }),
            author: None,           // Could extract from first message's From header
            created_at: updated_at, // Use latest message date as created date for threads
            updated_at,
            mime_type: Some("application/x-gmail-thread".to_string()),
            size: None, // Could calculate total thread size
            url: None,  // Could generate Gmail web URL
            path: Some(format!("/Gmail/{}", self.subject)),
            extra: Some(extra),
        };

        let permissions = DocumentPermissions {
            public: false,
            users: self.participants.iter().cloned().collect(),
            groups: vec![],
        };

        Ok(ConnectorEvent::DocumentCreated {
            sync_run_id: sync_run_id.to_string(),
            source_id: source_id.to_string(),
            document_id: self.thread_id.clone(),
            content_id: content_id.to_string(),
            metadata,
            permissions,
        })
    }
}
