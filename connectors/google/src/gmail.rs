use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

use crate::auth::{execute_with_auth_retry, is_auth_error, ApiResult, ServiceAccountAuth};
use shared::RateLimiter;

const GMAIL_API_BASE: &str = "https://gmail.googleapis.com/gmail/v1";

#[derive(Clone)]
pub struct GmailClient {
    client: Client,
    rate_limiter: Option<Arc<RateLimiter>>,
}

impl GmailClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            rate_limiter: None,
        }
    }

    pub fn with_rate_limiter(rate_limiter: Arc<RateLimiter>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(60))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            rate_limiter: Some(rate_limiter),
        }
    }

    pub async fn list_messages(
        &self,
        auth: &ServiceAccountAuth,
        user_email: &str,
        query: Option<&str>,
        max_results: Option<u32>,
        page_token: Option<&str>,
    ) -> Result<MessagesListResponse> {
        let query = query.map(|s| s.to_string());
        let page_token = page_token.map(|s| s.to_string());

        execute_with_auth_retry(auth, user_email, &self.rate_limiter, |token| {
            let query = query.clone();
            let page_token = page_token.clone();
            async move {
                let url = format!("{}/users/{}/messages", GMAIL_API_BASE, user_email);

                let mut params = vec![("maxResults", max_results.unwrap_or(100).to_string())];

                if let Some(ref q) = query {
                    params.push(("q", q.clone()));
                }

                if let Some(ref page_token) = page_token {
                    params.push(("pageToken", page_token.clone()));
                }

                let response = self
                    .client
                    .get(&url)
                    .bearer_auth(&token)
                    .query(&params)
                    .send()
                    .await?;

                let status = response.status();
                if is_auth_error(status) {
                    return Ok(ApiResult::AuthError);
                } else if !status.is_success() {
                    let error_text = response.text().await?;
                    return Ok(ApiResult::OtherError(anyhow!(
                        "Failed to list messages: HTTP {} - {}",
                        status,
                        error_text
                    )));
                }

                debug!("Gmail API list messages response status: {}", status);
                let response_text = response.text().await?;
                debug!("Gmail API raw response: {}", response_text);

                let parsed_response = serde_json::from_str(&response_text).map_err(|e| {
                    anyhow!(
                        "Failed to parse Gmail API response: {}. Raw response: {}",
                        e,
                        response_text
                    )
                })?;

                Ok(ApiResult::Success(parsed_response))
            }
        })
        .await
    }

    pub async fn get_message(
        &self,
        auth: &ServiceAccountAuth,
        user_email: &str,
        message_id: &str,
        format: MessageFormat,
    ) -> Result<GmailMessage> {
        let message_id = message_id.to_string();

        execute_with_auth_retry(auth, user_email, &self.rate_limiter, |token| {
            let message_id = message_id.clone();
            async move {
                let url = format!(
                    "{}/users/{}/messages/{}",
                    GMAIL_API_BASE, user_email, message_id
                );

                let format_str = match format {
                    MessageFormat::Full => "full",
                    MessageFormat::Metadata => "metadata",
                    MessageFormat::Minimal => "minimal",
                    MessageFormat::Raw => "raw",
                };

                let params = vec![("format", format_str)];

                let response = self
                    .client
                    .get(&url)
                    .bearer_auth(&token)
                    .query(&params)
                    .send()
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to send request to Gmail API for message {}",
                            message_id
                        )
                    })?;

                let status = response.status();
                if is_auth_error(status) {
                    return Ok(ApiResult::AuthError);
                } else if !status.is_success() {
                    let error_text = response.text().await?;
                    return Ok(ApiResult::OtherError(anyhow!(
                        "Gmail API returned error for message {}: HTTP {} - {}",
                        message_id,
                        status,
                        error_text
                    )));
                }

                debug!("Gmail API get message response status: {}", status);
                let response_text = response
                    .text()
                    .await
                    .context("Failed to read response body from Gmail API")?;

                let message: GmailMessage =
                    serde_json::from_str(&response_text).with_context(|| {
                        format!(
                            "Failed to parse Gmail API response for message {}. Raw response: {}",
                            message_id, response_text
                        )
                    })?;

                Ok(ApiResult::Success(message))
            }
        })
        .await
    }

    pub async fn list_history(
        &self,
        auth: &ServiceAccountAuth,
        user_email: &str,
        start_history_id: &str,
        max_results: Option<u32>,
        page_token: Option<&str>,
    ) -> Result<HistoryListResponse> {
        let start_history_id = start_history_id.to_string();
        let page_token = page_token.map(|s| s.to_string());

        execute_with_auth_retry(auth, user_email, &self.rate_limiter, |token| {
            let start_history_id = start_history_id.clone();
            let page_token = page_token.clone();
            async move {
                let url = format!("{}/users/{}/history", GMAIL_API_BASE, user_email);

                let mut params = vec![
                    ("startHistoryId", start_history_id),
                    ("maxResults", max_results.unwrap_or(100).to_string()),
                ];

                if let Some(ref page_token) = page_token {
                    params.push(("pageToken", page_token.clone()));
                }

                let response = self
                    .client
                    .get(&url)
                    .bearer_auth(&token)
                    .query(&params)
                    .send()
                    .await?;

                let status = response.status();
                if is_auth_error(status) {
                    return Ok(ApiResult::AuthError);
                } else if !status.is_success() {
                    let error_text = response.text().await?;
                    return Ok(ApiResult::OtherError(anyhow!(
                        "Failed to list history: HTTP {} - {}",
                        status,
                        error_text
                    )));
                }

                debug!("Gmail API list history response status: {}", status);
                let response_text = response.text().await?;

                let parsed_response = serde_json::from_str(&response_text).map_err(|e| {
                    anyhow!(
                        "Failed to parse Gmail history API response: {}. Raw response: {}",
                        e,
                        response_text
                    )
                })?;

                Ok(ApiResult::Success(parsed_response))
            }
        })
        .await
    }

    pub async fn get_profile(
        &self,
        auth: &ServiceAccountAuth,
        user_email: &str,
    ) -> Result<GmailProfile> {
        execute_with_auth_retry(auth, user_email, &self.rate_limiter, |token| async move {
            let url = format!("{}/users/{}/profile", GMAIL_API_BASE, user_email);

            let response = self.client.get(&url).bearer_auth(&token).send().await?;

            let status = response.status();
            if is_auth_error(status) {
                return Ok(ApiResult::AuthError);
            } else if !status.is_success() {
                let error_text = response.text().await?;
                return Ok(ApiResult::OtherError(anyhow!(
                    "Failed to get profile: HTTP {} - {}",
                    status,
                    error_text
                )));
            }

            let response_text = response.text().await?;
            let profile: GmailProfile = serde_json::from_str(&response_text)?;

            Ok(ApiResult::Success(profile))
        })
        .await
    }

    pub fn extract_message_content(&self, message: &GmailMessage) -> Result<String> {
        if let Some(ref payload) = message.payload {
            self.extract_text_from_payload(payload)
        } else {
            Ok(String::new())
        }
    }

    fn extract_text_from_payload(&self, payload: &MessagePart) -> Result<String> {
        let mut content = String::new();

        // If this part has a body with data, extract it
        if let Some(ref body) = payload.body {
            if let Some(ref data) = body.data {
                if let Some(mime_type) = &payload.mime_type {
                    if mime_type.starts_with("text/") {
                        match URL_SAFE_NO_PAD.decode(data) {
                            Ok(decoded) => {
                                if let Ok(text) = String::from_utf8(decoded) {
                                    if mime_type == "text/html" {
                                        // Simple HTML to text conversion
                                        content.push_str(&self.html_to_text(&text));
                                    } else {
                                        content.push_str(&text);
                                    }
                                }
                            }
                            Err(e) => {
                                debug!("Failed to decode message part: {}", e);
                            }
                        }
                    }
                }
            }
        }

        // Recursively process parts
        if let Some(ref parts) = payload.parts {
            for part in parts {
                if let Ok(part_content) = self.extract_text_from_payload(part) {
                    if !part_content.is_empty() {
                        content.push_str(&part_content);
                        content.push('\n');
                    }
                }
            }
        }

        Ok(content)
    }

    fn html_to_text(&self, html: &str) -> String {
        // Simple HTML tag removal - in production, consider using a proper HTML parser
        let re = regex::Regex::new(r"<[^>]*>").unwrap();
        let text = re.replace_all(html, " ");

        // Decode common HTML entities
        text.replace("&nbsp;", " ")
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
    }

    pub fn get_header_value(&self, message: &GmailMessage, header_name: &str) -> Option<String> {
        message
            .payload
            .as_ref()?
            .headers
            .as_ref()?
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case(header_name))
            .map(|h| h.value.clone())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MessageFormat {
    Full,
    Metadata,
    Minimal,
    Raw,
}

#[derive(Debug, Deserialize)]
pub struct MessagesListResponse {
    pub messages: Option<Vec<MessageInfo>>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    #[serde(rename = "resultSizeEstimate")]
    pub result_size_estimate: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct MessageInfo {
    pub id: String,
    #[serde(rename = "threadId")]
    pub thread_id: String,
}

#[derive(Debug, Deserialize)]
pub struct GmailMessage {
    pub id: String,
    #[serde(rename = "threadId")]
    pub thread_id: String,
    #[serde(rename = "labelIds")]
    pub label_ids: Option<Vec<String>>,
    pub snippet: Option<String>,
    #[serde(rename = "historyId")]
    pub history_id: Option<String>,
    #[serde(rename = "internalDate")]
    pub internal_date: Option<String>,
    pub payload: Option<MessagePart>,
    #[serde(rename = "sizeEstimate")]
    pub size_estimate: Option<u64>,
    pub raw: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MessagePart {
    #[serde(rename = "partId")]
    pub part_id: Option<String>,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    pub filename: Option<String>,
    pub headers: Option<Vec<Header>>,
    pub body: Option<MessagePartBody>,
    pub parts: Option<Vec<MessagePart>>,
}

#[derive(Debug, Deserialize)]
pub struct Header {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct MessagePartBody {
    #[serde(rename = "attachmentId")]
    pub attachment_id: Option<String>,
    pub size: Option<u64>,
    pub data: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryListResponse {
    pub history: Option<Vec<History>>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    #[serde(rename = "historyId")]
    pub history_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct History {
    pub id: String,
    pub messages: Option<Vec<HistoryMessage>>,
    #[serde(rename = "messagesAdded")]
    pub messages_added: Option<Vec<HistoryMessageAdded>>,
    #[serde(rename = "messagesDeleted")]
    pub messages_deleted: Option<Vec<HistoryMessageDeleted>>,
    #[serde(rename = "labelsAdded")]
    pub labels_added: Option<Vec<HistoryLabelAdded>>,
    #[serde(rename = "labelsRemoved")]
    pub labels_removed: Option<Vec<HistoryLabelRemoved>>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryMessage {
    pub id: String,
    #[serde(rename = "threadId")]
    pub thread_id: String,
}

#[derive(Debug, Deserialize)]
pub struct HistoryMessageAdded {
    pub message: HistoryMessage,
}

#[derive(Debug, Deserialize)]
pub struct HistoryMessageDeleted {
    pub message: HistoryMessage,
}

#[derive(Debug, Deserialize)]
pub struct HistoryLabelAdded {
    pub message: HistoryMessage,
    #[serde(rename = "labelIds")]
    pub label_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryLabelRemoved {
    pub message: HistoryMessage,
    #[serde(rename = "labelIds")]
    pub label_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct GmailProfile {
    #[serde(rename = "emailAddress")]
    pub email_address: String,
    #[serde(rename = "messagesTotal")]
    pub messages_total: Option<u64>,
    #[serde(rename = "threadsTotal")]
    pub threads_total: Option<u64>,
    #[serde(rename = "historyId")]
    pub history_id: String,
}
