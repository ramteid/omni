use anyhow::{anyhow, Result};
use reqwest::{Client, Response, StatusCode};
use serde::de::DeserializeOwned;
use std::time::Duration;
use tokio::time::{sleep, Instant};
use tracing::{debug, warn};

use crate::auth::AtlassianCredentials;
use crate::models::{ConfluencePage, ConfluenceSearchResponse, JiraIssue, JiraSearchResponse};

#[derive(Debug, Clone)]
pub struct RateLimitState {
    pub requests_remaining: Option<u32>,
    pub reset_time: Option<Instant>,
    pub retry_after: Option<Duration>,
}

impl Default for RateLimitState {
    fn default() -> Self {
        Self {
            requests_remaining: None,
            reset_time: None,
            retry_after: None,
        }
    }
}

pub struct AtlassianClient {
    client: Client,
    rate_limit_state: RateLimitState,
}

impl AtlassianClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Clio/1.0 (Atlassian Connector)")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            rate_limit_state: RateLimitState::default(),
        }
    }

    async fn make_request<T>(
        &mut self,
        request_fn: impl Fn() -> reqwest::RequestBuilder,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        const MAX_RETRIES: u32 = 5;
        const BASE_DELAY: Duration = Duration::from_millis(1000);

        for attempt in 0..MAX_RETRIES {
            // Check rate limit before making request
            if let Some(retry_after) = self.rate_limit_state.retry_after {
                warn!("Rate limited, waiting {} seconds", retry_after.as_secs());
                sleep(retry_after).await;
                self.rate_limit_state.retry_after = None;
            }

            let request = request_fn();
            let response = request.send().await?;

            // Update rate limit state from response headers
            self.update_rate_limit_state(&response);

            match response.status() {
                StatusCode::OK => {
                    let data: T = response.json().await?;
                    return Ok(data);
                }
                StatusCode::TOO_MANY_REQUESTS => {
                    let retry_after = self.extract_retry_after(&response);
                    self.rate_limit_state.retry_after = Some(retry_after);

                    warn!(
                        "Rate limited (429), attempt {}/{}, waiting {} seconds",
                        attempt + 1,
                        MAX_RETRIES,
                        retry_after.as_secs()
                    );

                    if attempt == MAX_RETRIES - 1 {
                        let error_text = response.text().await.unwrap_or_default();
                        return Err(anyhow!(
                            "Rate limit exceeded after {} retries: {}",
                            MAX_RETRIES,
                            error_text
                        ));
                    }

                    sleep(retry_after).await;
                    continue;
                }
                StatusCode::UNAUTHORIZED => {
                    let error_text = response.text().await.unwrap_or_default();
                    return Err(anyhow!("Authentication failed: {}", error_text));
                }
                StatusCode::FORBIDDEN => {
                    let error_text = response.text().await.unwrap_or_default();
                    return Err(anyhow!("Access forbidden: {}", error_text));
                }
                StatusCode::NOT_FOUND => {
                    let error_text = response.text().await.unwrap_or_default();
                    return Err(anyhow!("Resource not found: {}", error_text));
                }
                StatusCode::INTERNAL_SERVER_ERROR
                | StatusCode::BAD_GATEWAY
                | StatusCode::SERVICE_UNAVAILABLE
                | StatusCode::GATEWAY_TIMEOUT => {
                    let status = response.status();
                    let error_text = response.text().await.unwrap_or_default();

                    if attempt == MAX_RETRIES - 1 {
                        return Err(anyhow!(
                            "Server error after {} retries: HTTP {} - {}",
                            MAX_RETRIES,
                            status,
                            error_text
                        ));
                    }

                    let delay = BASE_DELAY * (2_u32.pow(attempt));
                    warn!(
                        "Server error ({}), attempt {}/{}, retrying in {} seconds: {}",
                        status,
                        attempt + 1,
                        MAX_RETRIES,
                        delay.as_secs(),
                        error_text
                    );

                    sleep(delay).await;
                    continue;
                }
                _ => {
                    let status = response.status();
                    let error_text = response.text().await.unwrap_or_default();
                    return Err(anyhow!("Unexpected HTTP status {}: {}", status, error_text));
                }
            }
        }

        Err(anyhow!("Max retries exceeded"))
    }

    fn update_rate_limit_state(&mut self, response: &Response) {
        // Atlassian uses X-RateLimit headers
        if let Some(remaining) = response.headers().get("X-RateLimit-Remaining") {
            if let Ok(remaining_str) = remaining.to_str() {
                if let Ok(remaining_val) = remaining_str.parse::<u32>() {
                    self.rate_limit_state.requests_remaining = Some(remaining_val);
                }
            }
        }

        if let Some(reset) = response.headers().get("X-RateLimit-Reset") {
            if let Ok(reset_str) = reset.to_str() {
                if let Ok(reset_timestamp) = reset_str.parse::<u64>() {
                    let reset_time = Instant::now() + Duration::from_secs(reset_timestamp);
                    self.rate_limit_state.reset_time = Some(reset_time);
                }
            }
        }
    }

    fn extract_retry_after(&self, response: &Response) -> Duration {
        if let Some(retry_after) = response.headers().get("Retry-After") {
            if let Ok(retry_after_str) = retry_after.to_str() {
                if let Ok(seconds) = retry_after_str.parse::<u64>() {
                    return Duration::from_secs(seconds);
                }
            }
        }

        // Default retry delay if header is missing
        Duration::from_secs(60)
    }

    pub async fn get_confluence_pages(
        &mut self,
        creds: &AtlassianCredentials,
        space_key: Option<&str>,
        limit: u32,
        start: u32,
        expand: &[&str],
    ) -> Result<ConfluenceSearchResponse> {
        let auth_header = creds.get_basic_auth_header();
        let mut url = format!("{}/wiki/rest/api/content", creds.base_url);
        let mut params = vec![
            ("limit", limit.to_string()),
            ("start", start.to_string()),
            ("type", "page".to_string()),
        ];

        if let Some(space) = space_key {
            params.push(("spaceKey", space.to_string()));
        }

        if !expand.is_empty() {
            params.push(("expand", expand.join(",")));
        }

        let query_string = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        url.push_str(&format!("?{}", query_string));

        debug!("Fetching Confluence pages from: {}", url);

        let client = self.client.clone();
        self.make_request(move || {
            client
                .get(&url)
                .header("Authorization", &auth_header)
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
        })
        .await
    }

    pub async fn get_confluence_page_by_id(
        &mut self,
        creds: &AtlassianCredentials,
        page_id: &str,
        expand: &[&str],
    ) -> Result<ConfluencePage> {
        let auth_header = creds.get_basic_auth_header();
        let mut url = format!("{}/wiki/rest/api/content/{}", creds.base_url, page_id);

        if !expand.is_empty() {
            url.push_str(&format!("?expand={}", expand.join(",")));
        }

        debug!("Fetching Confluence page: {}", url);

        let client = self.client.clone();
        self.make_request(move || {
            client
                .get(&url)
                .header("Authorization", &auth_header)
                .header("Accept", "application/json")
        })
        .await
    }

    pub async fn get_confluence_pages_updated_since(
        &mut self,
        creds: &AtlassianCredentials,
        since: &str,
        limit: u32,
        start: u32,
    ) -> Result<ConfluenceSearchResponse> {
        let auth_header = creds.get_basic_auth_header();
        let cql = format!("lastModified >= '{}'", since);
        let url = format!(
            "{}/wiki/rest/api/content/search?cql={}&limit={}&start={}&expand=body.storage,space,version,ancestors",
            creds.base_url,
            urlencoding::encode(&cql),
            limit,
            start
        );

        debug!(
            "Searching Confluence pages updated since {}: {}",
            since, url
        );

        let client = self.client.clone();
        self.make_request(move || {
            client
                .get(&url)
                .header("Authorization", &auth_header)
                .header("Accept", "application/json")
        })
        .await
    }

    pub async fn get_jira_issues(
        &mut self,
        creds: &AtlassianCredentials,
        jql: &str,
        max_results: u32,
        start_at: u32,
        fields: &[&str],
    ) -> Result<JiraSearchResponse> {
        let auth_header = creds.get_basic_auth_header();
        let url = format!("{}/rest/api/3/search", creds.base_url);

        let request_body = serde_json::json!({
            "jql": jql,
            "startAt": start_at,
            "maxResults": max_results,
            "fields": fields,
            "expand": ["renderedFields"]
        });

        debug!("Searching JIRA issues with JQL: {}", jql);

        let client = self.client.clone();
        self.make_request(move || {
            client
                .post(&url)
                .header("Authorization", &auth_header)
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .json(&request_body)
        })
        .await
    }

    pub async fn get_jira_issues_updated_since(
        &mut self,
        creds: &AtlassianCredentials,
        since: &str,
        project_key: Option<&str>,
        max_results: u32,
        start_at: u32,
    ) -> Result<JiraSearchResponse> {
        let mut jql = format!("updated >= '{}'", since);

        if let Some(project) = project_key {
            jql = format!("project = {} AND {}", project, jql);
        }

        let fields = vec![
            "summary",
            "description",
            "issuetype",
            "status",
            "priority",
            "assignee",
            "reporter",
            "creator",
            "project",
            "created",
            "updated",
            "labels",
            "comment",
            "components",
        ];

        self.get_jira_issues(creds, &jql, max_results, start_at, &fields)
            .await
    }

    pub async fn get_jira_issue_by_key(
        &mut self,
        creds: &AtlassianCredentials,
        issue_key: &str,
        fields: &[&str],
    ) -> Result<JiraIssue> {
        let auth_header = creds.get_basic_auth_header();
        let fields_param = if fields.is_empty() {
            "*all".to_string()
        } else {
            fields.join(",")
        };

        let url = format!(
            "{}/rest/api/3/issue/{}?fields={}&expand=renderedFields",
            creds.base_url,
            issue_key,
            urlencoding::encode(&fields_param)
        );

        debug!("Fetching JIRA issue: {}", url);

        let client = self.client.clone();
        self.make_request(move || {
            client
                .get(&url)
                .header("Authorization", &auth_header)
                .header("Accept", "application/json")
        })
        .await
    }

    pub async fn get_confluence_spaces(
        &mut self,
        creds: &AtlassianCredentials,
        limit: u32,
        start: u32,
    ) -> Result<serde_json::Value> {
        let auth_header = creds.get_basic_auth_header();
        let url = format!(
            "{}/wiki/rest/api/space?limit={}&start={}",
            creds.base_url, limit, start
        );

        debug!("Fetching Confluence spaces: {}", url);

        let client = self.client.clone();
        self.make_request(move || {
            client
                .get(&url)
                .header("Authorization", &auth_header)
                .header("Accept", "application/json")
        })
        .await
    }

    pub async fn get_jira_projects(
        &mut self,
        creds: &AtlassianCredentials,
        expand: &[&str],
    ) -> Result<Vec<serde_json::Value>> {
        let auth_header = creds.get_basic_auth_header();
        let mut url = format!("{}/rest/api/3/project", creds.base_url);

        if !expand.is_empty() {
            url.push_str(&format!("?expand={}", expand.join(",")));
        }

        debug!("Fetching JIRA projects: {}", url);

        let client = self.client.clone();
        self.make_request(move || {
            client
                .get(&url)
                .header("Authorization", &auth_header)
                .header("Accept", "application/json")
        })
        .await
    }

    pub fn get_rate_limit_info(&self) -> &RateLimitState {
        &self.rate_limit_state
    }
}
