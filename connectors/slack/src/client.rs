use anyhow::{anyhow, Result};
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use crate::models::{
    ConversationsHistoryResponse, ConversationsListResponse, SlackFile, UsersListResponse,
};

const SLACK_API_BASE: &str = "https://slack.com/api";
const RATE_LIMIT_DELAY_MS: u64 = 1000; // 1 second between requests

pub struct SlackClient {
    client: Client,
}

impl SlackClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    async fn make_request<T>(&self, url: &str, token: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        debug!("Making request to: {}", url);

        // Rate limiting - sleep before each request
        sleep(Duration::from_millis(RATE_LIMIT_DELAY_MS)).await;

        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("API request failed: {}", error_text));
        }

        let response_text = response.text().await?;
        debug!("Response: {}", response_text);

        let parsed: T = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

        Ok(parsed)
    }

    pub async fn list_conversations(
        &self,
        token: &str,
        cursor: Option<&str>,
    ) -> Result<ConversationsListResponse> {
        let mut url = format!(
            "{}/conversations.list?types=public_channel,private_channel&limit=200",
            SLACK_API_BASE
        );

        if let Some(cursor) = cursor {
            url.push_str(&format!("&cursor={}", cursor));
        }

        let response: ConversationsListResponse = self.make_request(&url, token).await?;

        if !response.ok {
            return Err(anyhow!(
                "conversations.list failed: {}",
                response.error.unwrap_or("Unknown error".to_string())
            ));
        }

        info!("Found {} channels", response.channels.len());
        Ok(response)
    }

    pub async fn get_conversation_history(
        &self,
        token: &str,
        channel_id: &str,
        cursor: Option<&str>,
        oldest: Option<&str>,
        latest: Option<&str>,
    ) -> Result<ConversationsHistoryResponse> {
        let mut url = format!(
            "{}/conversations.history?channel={}&limit=200",
            SLACK_API_BASE, channel_id
        );

        if let Some(cursor) = cursor {
            url.push_str(&format!("&cursor={}", cursor));
        }
        if let Some(oldest) = oldest {
            url.push_str(&format!("&oldest={}", oldest));
        }
        if let Some(latest) = latest {
            url.push_str(&format!("&latest={}", latest));
        }

        let response: ConversationsHistoryResponse = self.make_request(&url, token).await?;

        if !response.ok {
            return Err(anyhow!(
                "conversations.history failed: {}",
                response.error.unwrap_or("Unknown error".to_string())
            ));
        }

        debug!(
            "Retrieved {} messages from channel {}",
            response.messages.len(),
            channel_id
        );
        Ok(response)
    }

    pub async fn get_thread_replies(
        &self,
        token: &str,
        channel_id: &str,
        thread_ts: &str,
        cursor: Option<&str>,
    ) -> Result<ConversationsHistoryResponse> {
        let mut url = format!(
            "{}/conversations.replies?channel={}&ts={}&limit=200",
            SLACK_API_BASE, channel_id, thread_ts
        );

        if let Some(cursor) = cursor {
            url.push_str(&format!("&cursor={}", cursor));
        }

        let response: ConversationsHistoryResponse = self.make_request(&url, token).await?;

        if !response.ok {
            return Err(anyhow!(
                "conversations.replies failed: {}",
                response.error.unwrap_or("Unknown error".to_string())
            ));
        }

        debug!(
            "Retrieved {} thread replies for ts {}",
            response.messages.len(),
            thread_ts
        );
        Ok(response)
    }

    pub async fn list_users(&self, token: &str, cursor: Option<&str>) -> Result<UsersListResponse> {
        let mut url = format!("{}/users.list?limit=200", SLACK_API_BASE);

        if let Some(cursor) = cursor {
            url.push_str(&format!("&cursor={}", cursor));
        }

        let response: UsersListResponse = self.make_request(&url, token).await?;

        if !response.ok {
            return Err(anyhow!(
                "users.list failed: {}",
                response.error.unwrap_or("Unknown error".to_string())
            ));
        }

        info!("Found {} users", response.members.len());
        Ok(response)
    }

    pub async fn download_file(&self, token: &str, file: &SlackFile) -> Result<String> {
        if let Some(download_url) = &file.url_private_download {
            debug!("Downloading file: {} ({})", file.name, file.id);

            // Rate limiting
            sleep(Duration::from_millis(RATE_LIMIT_DELAY_MS)).await;

            let response = self
                .client
                .get(download_url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await?;

            if !response.status().is_success() {
                warn!(
                    "Failed to download file {}: HTTP {}",
                    file.name,
                    response.status()
                );
                return Ok(String::new());
            }

            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|ct| ct.to_str().ok())
                .unwrap_or("");

            // Only process text files for now
            if content_type.starts_with("text/") {
                let content = response.text().await?;
                return Ok(content);
            } else {
                debug!("Skipping non-text file: {} ({})", file.name, content_type);
                return Ok(String::new());
            }
        }

        Ok(String::new())
    }
}
